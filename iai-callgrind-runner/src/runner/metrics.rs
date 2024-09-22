use std::borrow::Cow;
use std::fmt::Display;
use std::hash::Hash;

use anyhow::{Context, Result};
use indexmap::map::Iter;
use indexmap::{IndexMap, IndexSet};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub trait Summarize: Hash + Eq + Clone {
    fn summarize(_: &mut Cow<Metrics<Self>>) {}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Metrics<K: Hash + Eq>(pub IndexMap<K, u64>);

impl<K: Hash + Eq + Display + Clone> Metrics<K> {
    /// Return empty `Metrics`
    pub fn empty() -> Self {
        Metrics(IndexMap::new())
    }

    // The order matters. The index is derived from the insertion order
    pub fn with_metric_kinds<T>(kinds: T) -> Self
    where
        T: IntoIterator<Item = (K, u64)>,
    {
        Self(kinds.into_iter().collect())
    }

    /// Add metrics from an iterator over strings
    ///
    /// Adding metrics stops as soon as there are no more keys in this `Metrics` or no more values
    /// in the iterator. This property is especially important for the metrics from the callgrind
    /// output files. From the documentation of the callgrind format:
    ///
    /// > If a cost line specifies less event counts than given in the "events" line, the
    /// > rest is assumed to be zero.
    ///
    /// # Errors
    ///
    /// If one of the strings in the iterator is not parsable as u64
    pub fn add_iter_str<I, T>(&mut self, iter: T) -> Result<()>
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        for (this, other) in self.0.values_mut().zip(iter.into_iter()) {
            *this += other
                .as_ref()
                .parse::<u64>()
                .context("A metric must be an integer type")?;
        }

        Ok(())
    }

    /// Sum this `Metric` with another `Metric`
    ///
    /// Do not use this method if both `Metrics` can differ in their keys order.
    pub fn add(&mut self, other: &Self) {
        for (this, other) in self.0.values_mut().zip(other.0.values()) {
            *this += other;
        }
    }

    /// Return the metric of the kind at index (of insertion order) if present
    ///
    /// This operation is O(1)
    pub fn metric_by_index(&self, index: usize) -> Option<u64> {
        self.0.get_index(index).map(|(_, c)| *c)
    }

    /// Return the metric of the `kind` if present
    ///
    /// This operation is O(1)
    pub fn metric_by_kind(&self, kind: &K) -> Option<u64> {
        self.0.get_key_value(kind).map(|(_, c)| *c)
    }

    pub fn try_metric_by_kind(&self, kind: &K) -> Result<u64> {
        self.metric_by_kind(kind)
            .with_context(|| format!("Missing event type '{kind}"))
    }

    pub fn metric_kinds(&self) -> Vec<K> {
        self.0.iter().map(|(k, _)| k.clone()).collect()
    }

    /// Create the union set of the keys of this and another `Metrics`
    ///
    /// The order of the keys is preserved. New keys from the `other` Metrics are appended in their
    /// original order.
    pub fn metric_kinds_union<'a>(&'a self, other: &'a Self) -> IndexSet<&'a K> {
        let set = self.0.keys().collect::<IndexSet<_>>();
        let other_set = other.0.keys().collect::<IndexSet<_>>();
        set.union(&other_set).copied().collect()
    }

    pub fn iter(&self) -> Iter<'_, K, u64> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn insert(&mut self, key: K, value: u64) -> Option<u64> {
        self.0.insert(key, value)
    }

    pub fn insert_all(&mut self, entries: &[(K, u64)]) {
        for (key, value) in entries {
            self.insert(key.clone(), *value);
        }
    }
}

impl<'a, K: Hash + Eq + Display + Clone> IntoIterator for &'a Metrics<K> {
    type Item = (&'a K, &'a u64);

    type IntoIter = Iter<'a, K, u64>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<I, K: Hash + Eq + From<I>> FromIterator<I> for Metrics<K> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = I>,
    {
        Self(
            iter.into_iter()
                .map(|s| (K::from(s), 0))
                .collect::<IndexMap<_, _>>(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::iter;

    use rstest::rstest;

    use super::*;
    use crate::api::EventKind::{self, *};

    fn expected_metrics<T>(events: T) -> Metrics<EventKind>
    where
        T: IntoIterator<Item = (EventKind, u64)>,
    {
        Metrics(IndexMap::from_iter(events))
    }

    #[rstest]
    #[case::single_zero(&[Ir], &["0"], expected_metrics([(Ir, 0)]))]
    #[case::single_one(&[Ir], &["1"], expected_metrics([(Ir, 1)]))]
    #[case::single_u64_max(&[Ir], &[u64::MAX.to_string()], expected_metrics([(Ir, u64::MAX)]))]
    #[case::more_values_than_kinds(&[Ir], &["1", "2"], expected_metrics([(Ir, 1)]))]
    #[case::more_kinds_than_values(&[Ir, I1mr], &["1"], expected_metrics([(Ir, 1), (I1mr, 0)]))]
    fn test_metrics_add_iter_str<I>(
        #[case] event_kinds: &[EventKind],
        #[case] to_add: &[I],
        #[case] expected_metrics: Metrics<EventKind>,
    ) where
        I: AsRef<str>,
    {
        let mut metrics =
            Metrics::with_metric_kinds(event_kinds.iter().copied().zip(iter::repeat(0)));
        metrics.add_iter_str(to_add).unwrap();

        assert_eq!(metrics, expected_metrics);
    }

    #[rstest]
    #[case::float(&[Ir], &["0.0"])]
    #[case::word(&[Ir], &["abc"])]
    #[case::empty(&[Ir], &[""])]
    #[case::one_more_than_max_u64(&[Ir], &["18446744073709551616"])]
    fn test_metrics_add_iter_str_when_error<I>(
        #[case] event_kinds: &[EventKind],
        #[case] to_add: &[I],
    ) where
        I: AsRef<str>,
    {
        let mut metrics =
            Metrics::with_metric_kinds(event_kinds.iter().copied().zip(iter::repeat(0)));
        assert!(metrics.add_iter_str(to_add).is_err());
    }
}
