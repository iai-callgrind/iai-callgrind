#![allow(clippy::cast_precision_loss)]

use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::{Add, AddAssign, Div, Mul, Sub};
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use indexmap::map::Iter;
use indexmap::{IndexMap, IndexSet};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::util::to_string_unsigned_short;
// TODO: Move other related structures like MetricsDiff etc. into this module

pub trait Summarize: Hash + Eq + Clone {
    fn summarize(_: &mut Cow<Metrics<Self>>) {}
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum Metric {
    Int(u64),
    Float(f64),
}

/// The `Metrics` backed by an [`indexmap::IndexMap`]
///
/// The insertion order is preserved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Metrics<K: Hash + Eq>(pub IndexMap<K, Metric>);

impl Add for Metric {
    type Output = Metric;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Metric::Int(a), Metric::Int(b)) => Self::Int(a.saturating_add(b)),
            (Metric::Int(a), Metric::Float(b)) => Self::Float((a as f64) + b),
            (Metric::Float(a), Metric::Int(b)) => Self::Float((b as f64) + a),
            (Metric::Float(a), Metric::Float(b)) => Self::Float(a + b),
        }
    }
}

impl AddAssign for Metric {
    fn add_assign(&mut self, rhs: Self) {
        let metric = match (&self, rhs) {
            (Metric::Int(a), Metric::Int(b)) => Metric::Int(a.saturating_add(b)),
            (Metric::Int(a), Metric::Float(b)) => {
                let c = (*a as f64) + b;
                Metric::Float(c)
            }
            (Metric::Float(a), Metric::Int(b)) => Metric::Float(a + b as f64),
            (Metric::Float(a), Metric::Float(b)) => Metric::Float(a + b),
        };

        *self = metric;
    }
}

impl Display for Metric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Metric::Int(a) => f.pad(&format!("{a}")),
            Metric::Float(a) => f.pad(&to_string_unsigned_short(*a)),
        }
    }
}

impl Div for Metric {
    type Output = Metric;

    // TODO: test and check result for division by 0 etc.
    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Metric::Int(a), Metric::Int(b)) => Metric::Float((a as f64) / (b as f64)),
            (Metric::Int(a), Metric::Float(b)) => Metric::Float((a as f64) / b),
            (Metric::Float(a), Metric::Int(b)) => Metric::Float(a / (b as f64)),
            (Metric::Float(a), Metric::Float(b)) => Metric::Float(a / b),
        }
    }
}

impl Eq for Metric {}

impl From<u64> for Metric {
    fn from(value: u64) -> Self {
        Metric::Int(value)
    }
}

impl From<f64> for Metric {
    fn from(value: f64) -> Self {
        Metric::Float(value)
    }
}

impl From<Metric> for f64 {
    fn from(value: Metric) -> Self {
        match value {
            Metric::Int(a) => a as f64,
            Metric::Float(a) => a,
        }
    }
}

impl FromStr for Metric {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.parse::<u64>() {
            Ok(a) => Ok(Metric::Int(a)),
            Err(_) => match s.parse::<f64>() {
                Ok(a) => Ok(Metric::Float(a)),
                Err(error) => Err(anyhow!("Invalid metric: {error}")),
            },
        }
    }
}

impl Mul<u64> for Metric {
    type Output = Metric;

    fn mul(self, rhs: u64) -> Self::Output {
        match self {
            Metric::Int(a) => Metric::Int(a.saturating_mul(rhs)),
            Metric::Float(a) => Metric::Float(a * (rhs as f64)),
        }
    }
}

impl Ord for Metric {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a.cmp(b),
            (Self::Int(a), Self::Float(b)) => (*a as f64).total_cmp(b),
            (Self::Float(a), Self::Int(b)) => a.total_cmp(&(*b as f64)),
            (Self::Float(a), Self::Float(b)) => a.total_cmp(b),
        }
    }
}

impl PartialEq for Metric {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Int(a), Self::Float(b)) => (*a as f64).total_cmp(b) == Ordering::Equal,
            (Self::Float(a), Self::Int(b)) => a.total_cmp(&(*b as f64)) == Ordering::Equal,
            (Self::Float(a), Self::Float(b)) => a.total_cmp(b) == Ordering::Equal,
        }
    }
}

impl PartialOrd for Metric {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Sub for Metric {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Metric::Int(a), Metric::Int(b)) => Metric::Int(a.saturating_sub(b)),
            (Metric::Int(a), Metric::Float(b)) => Metric::Float((a as f64) - b),
            (Metric::Float(a), Metric::Int(b)) => Metric::Float(a - (b as f64)),
            (Metric::Float(a), Metric::Float(b)) => Metric::Float(a - b),
        }
    }
}

impl<K: Hash + Eq + Display + Clone> Metrics<K> {
    /// Return empty `Metrics`
    pub fn empty() -> Self {
        Metrics(IndexMap::new())
    }

    // The order matters. The index is derived from the insertion order
    pub fn with_metric_kinds<I, T>(kinds: T) -> Self
    where
        I: Into<Metric>,
        T: IntoIterator<Item = (K, I)>,
    {
        Self(kinds.into_iter().map(|(k, n)| (k, n.into())).collect())
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
        // TODO: try to parse as f64 too. Adjust error message
        for (this, other) in self.0.values_mut().zip(iter.into_iter()) {
            *this += other
                .as_ref()
                .parse::<u64>()
                .map(Into::into)
                .context("A metric must be an integer type")?;
        }

        Ok(())
    }

    /// Sum this `Metric` with another `Metric`
    ///
    /// Do not use this method if both `Metrics` can differ in their keys order.
    pub fn add(&mut self, other: &Self) {
        for (this, other) in self.0.values_mut().zip(other.0.values()) {
            *this += *other;
        }
    }

    /// Return the metric of the kind at index (of insertion order) if present
    ///
    /// This operation is O(1)
    pub fn metric_by_index(&self, index: usize) -> Option<Metric> {
        self.0.get_index(index).map(|(_, c)| *c)
    }

    /// Return the metric of the `kind` if present
    ///
    /// This operation is O(1)
    pub fn metric_by_kind(&self, kind: &K) -> Option<Metric> {
        self.0.get_key_value(kind).map(|(_, c)| *c)
    }

    /// Return the metric kind or an error
    ///
    /// # Errors
    ///
    /// If the metric kind is not present
    pub fn try_metric_by_kind(&self, kind: &K) -> Result<Metric> {
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

    /// Return an iterator over the metrics in insertion order
    pub fn iter(&self) -> Iter<'_, K, Metric> {
        self.0.iter()
    }

    /// Return true if there are no metrics present
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Insert a single metric
    ///
    /// If an equivalent key already exists in the map: the key remains and retains in its place in
    /// the order, its corresponding value is updated with `value`, and the older value is returned
    /// inside `Some(_)`.
    ///
    /// If no equivalent key existed in the map: the new key-value pair is inserted, last in order,
    /// and `None` is returned.
    pub fn insert(&mut self, key: K, value: Metric) -> Option<Metric> {
        self.0.insert(key, value)
    }

    /// Insert all metrics
    ///
    /// See also [`Metrics::insert`]
    pub fn insert_all(&mut self, entries: &[(K, Metric)]) {
        for (key, value) in entries {
            self.insert(key.clone(), *value);
        }
    }
}

impl<'a, K: Hash + Eq + Display + Clone> IntoIterator for &'a Metrics<K> {
    type Item = (&'a K, &'a Metric);

    type IntoIter = Iter<'a, K, Metric>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// TODO: When is this impl used
// TODO: Metric::Int(0) or  Metric::Float(0.0)
impl<I, K: Hash + Eq + From<I>> FromIterator<I> for Metrics<K> {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = I>,
    {
        Self(
            iter.into_iter()
                .map(|s| (K::from(s), Metric::Int(0)))
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

    // TODO: Add tests for all Metric::Int, Metric::Float, ... and AddAssign, Add, ...
    fn expected_metrics<T>(events: T) -> Metrics<EventKind>
    where
        T: IntoIterator<Item = (EventKind, u64)>,
    {
        Metrics(
            events
                .into_iter()
                .map(|(k, n)| (k, n.into()))
                .collect::<IndexMap<_, _>>(),
        )
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
