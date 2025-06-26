#![allow(clippy::cast_precision_loss)]

use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::{Add, AddAssign, Div, Mul, Sub};
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use indexmap::map::Iter;
use indexmap::{indexmap, IndexMap, IndexSet};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::summary::Diffs;
use crate::api::{CachegrindMetric, DhatMetric, ErrorMetric, EventKind};
use crate::util::{to_string_unsigned_short, EitherOrBoth};

pub trait Summarize: Hash + Eq + Clone {
    fn summarize(_: &mut Cow<Metrics<Self>>) {}
}

/// The metric measured by valgrind or derived from one or more other metrics
///
/// The valgrind metrics measured by any of its tools are `u64`. However, to be able to represent
/// derived metrics like cache miss/hit rates it is inevitable to have a type which can store a
/// `u64` or a `f64`. When doing math with metrics, the original type should be preserved as far as
/// possible by using `u64` operations. A float metric should be a last resort.
///
/// Float operations with a `Metric` that stores a `u64` introduce a precision loss and are to be
/// avoided. Especially comparison between a `u64` metric and `f64` metric are not exact because the
/// `u64` has to be converted to a `f64`. Also, if adding/multiplying two `u64` metrics would result
/// in an overflow the metric saturates at `u64::MAX`. This choice was made to preserve precision
/// and the original type (instead of for example adding the two `u64` by converting both of them to
/// `f64`).
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum Metric {
    Int(u64),
    Float(f64),
}

/// The different metrics distinguished by tool and if it is an error checking tool as `ErrorMetric`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum MetricKind {
    None,
    Callgrind(EventKind),
    Cachegrind(CachegrindMetric),
    Dhat(DhatMetric),
    Memcheck(ErrorMetric),
    Helgrind(ErrorMetric),
    DRD(ErrorMetric),
}

/// The `Metrics` backed by an [`indexmap::IndexMap`]
///
/// The insertion order is preserved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Metrics<K: Hash + Eq>(pub IndexMap<K, Metric>);

/// The `MetricsDiff` describes the difference between a `new` and `old` metric as percentage and
/// factor.
///
/// Only if both metrics are present there is also a `Diffs` present. Otherwise, it just stores the
/// `new` or `old` metric.
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct MetricsDiff {
    /// Either the `new`, `old` or both metrics
    pub metrics: EitherOrBoth<Metric>,
    /// If both metrics are present there is also a `Diffs` present
    pub diffs: Option<Diffs>,
}

/// The `MetricsSummary` contains all differences between two tool run segments
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct MetricsSummary<K: Hash + Eq = EventKind>(IndexMap<K, MetricsDiff>);

impl Metric {
    /// Divide by `rhs` normally but if rhs is `0` the result is by convention `0.0`
    ///
    /// No difference is made between negative 0.0 and positive 0.0 os rhs value. The result is
    /// always positive 0.0.
    pub fn div0(self, rhs: Self) -> Self {
        match (self, rhs) {
            (_, Metric::Int(0) | Metric::Float(0.0f64)) => Metric::Float(0.0f64),
            (a, b) => a / b,
        }
    }
}

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
        *self = *self + rhs;
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

impl Mul<Metric> for u64 {
    type Output = Metric;

    fn mul(self, rhs: Metric) -> Self::Output {
        match rhs {
            Metric::Int(b) => Metric::Int(self.saturating_mul(b)),
            Metric::Float(b) => Metric::Float((self as f64) * b),
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

impl Display for MetricKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricKind::None => Ok(()),
            MetricKind::Callgrind(metric) => f.write_fmt(format_args!("Callgrind: {metric}")),
            MetricKind::Cachegrind(metric) => f.write_fmt(format_args!("Cachegrind: {metric}")),
            MetricKind::Dhat(metric) => f.write_fmt(format_args!("DHAT: {metric}")),
            MetricKind::Memcheck(metric) => f.write_fmt(format_args!("Memcheck: {metric}")),
            MetricKind::Helgrind(metric) => f.write_fmt(format_args!("Helgrind: {metric}")),
            MetricKind::DRD(metric) => f.write_fmt(format_args!("DRD: {metric}")),
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
    /// If one of the strings in the iterator is not parsable as u64 or f64
    pub fn add_iter_str<I, T>(&mut self, iter: T) -> Result<()>
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        for (this, other) in self.0.values_mut().zip(iter.into_iter()) {
            *this += other
                .as_ref()
                .parse::<Metric>()
                .context("A metric must be a valid number")?;
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

impl MetricsDiff {
    pub fn new(metrics: EitherOrBoth<Metric>) -> Self {
        if let EitherOrBoth::Both(new, old) = metrics {
            Self {
                metrics,
                diffs: Some(Diffs::new(new, old)),
            }
        } else {
            Self {
                metrics,
                diffs: None,
            }
        }
    }

    pub fn add(&self, other: &Self) -> Self {
        match (&self.metrics, &other.metrics) {
            (EitherOrBoth::Left(new), EitherOrBoth::Left(other_new)) => {
                Self::new(EitherOrBoth::Left(*new + *other_new))
            }
            (EitherOrBoth::Right(old), EitherOrBoth::Left(new))
            | (EitherOrBoth::Left(new), EitherOrBoth::Right(old)) => {
                Self::new(EitherOrBoth::Both(*new, *old))
            }
            (EitherOrBoth::Right(old), EitherOrBoth::Right(other_old)) => {
                Self::new(EitherOrBoth::Right(*old + *other_old))
            }
            (EitherOrBoth::Both(new, old), EitherOrBoth::Left(other_new))
            | (EitherOrBoth::Left(new), EitherOrBoth::Both(other_new, old)) => {
                Self::new(EitherOrBoth::Both(*new + *other_new, *old))
            }
            (EitherOrBoth::Both(new, old), EitherOrBoth::Right(other_old))
            | (EitherOrBoth::Right(old), EitherOrBoth::Both(new, other_old)) => {
                Self::new(EitherOrBoth::Both(*new, *old + *other_old))
            }
            (EitherOrBoth::Both(new, old), EitherOrBoth::Both(other_new, other_old)) => {
                Self::new(EitherOrBoth::Both(*new + *other_new, *old + *other_old))
            }
        }
    }
}

impl<K> MetricsSummary<K>
where
    K: Hash + Eq + Summarize + Display + Clone,
{
    /// Create a new `MetricsSummary` calculating the differences between new and old (if any)
    /// [`Metrics`]
    ///
    /// # Panics
    ///
    /// If one of the [`Metrics`] is empty
    pub fn new(metrics: EitherOrBoth<Metrics<K>>) -> Self {
        match metrics {
            EitherOrBoth::Left(new) => {
                assert!(!new.is_empty());

                let mut new = Cow::Owned(new);
                K::summarize(&mut new);

                Self(
                    new.iter()
                        .map(|(metric_kind, metric)| {
                            (
                                metric_kind.clone(),
                                MetricsDiff::new(EitherOrBoth::Left(*metric)),
                            )
                        })
                        .collect::<IndexMap<_, _>>(),
                )
            }
            EitherOrBoth::Right(old) => {
                assert!(!old.is_empty());

                let mut old = Cow::Owned(old);
                K::summarize(&mut old);

                Self(
                    old.iter()
                        .map(|(metric_kind, metric)| {
                            (
                                metric_kind.clone(),
                                MetricsDiff::new(EitherOrBoth::Right(*metric)),
                            )
                        })
                        .collect::<IndexMap<_, _>>(),
                )
            }
            EitherOrBoth::Both(new, old) => {
                assert!(!new.is_empty());
                assert!(!old.is_empty());

                let mut new = Cow::Owned(new);
                K::summarize(&mut new);
                let mut old = Cow::Owned(old);
                K::summarize(&mut old);

                let mut map = indexmap! {};
                for metric_kind in new.metric_kinds_union(&old) {
                    let diff = match (
                        new.metric_by_kind(metric_kind),
                        old.metric_by_kind(metric_kind),
                    ) {
                        (Some(metric), None) => MetricsDiff::new(EitherOrBoth::Left(metric)),
                        (None, Some(metric)) => MetricsDiff::new(EitherOrBoth::Right(metric)),
                        (Some(new), Some(old)) => MetricsDiff::new(EitherOrBoth::Both(new, old)),
                        (None, None) => {
                            unreachable!(
                                "The union contains the event kinds either from new or old or \
                                 from both"
                            )
                        }
                    };
                    map.insert(metric_kind.clone(), diff);
                }
                Self(map)
            }
        }
    }

    /// Try to return a [`MetricsDiff`] for the specified `MetricKind`
    pub fn diff_by_kind(&self, metric_kind: &K) -> Option<&MetricsDiff> {
        self.0.get(metric_kind)
    }

    pub fn all_diffs(&self) -> impl Iterator<Item = (&K, &MetricsDiff)> {
        self.0.iter()
    }

    pub fn extract_costs(&self) -> EitherOrBoth<Metrics<K>> {
        let mut new_metrics: Metrics<K> = Metrics::empty();
        let mut old_metrics: Metrics<K> = Metrics::empty();

        // The diffs should not be empty
        for (metric_kind, diff) in self.all_diffs() {
            match diff.metrics {
                EitherOrBoth::Left(new) => {
                    new_metrics.insert(metric_kind.clone(), new);
                }
                EitherOrBoth::Right(old) => {
                    old_metrics.insert(metric_kind.clone(), old);
                }
                EitherOrBoth::Both(new, old) => {
                    new_metrics.insert(metric_kind.clone(), new);
                    old_metrics.insert(metric_kind.clone(), old);
                }
            }
        }

        match (new_metrics.is_empty(), old_metrics.is_empty()) {
            (false, false) => EitherOrBoth::Both(new_metrics, old_metrics),
            (false, true) => EitherOrBoth::Left(new_metrics),
            (true, false) => EitherOrBoth::Right(old_metrics),
            (true, true) => unreachable!("A costs diff contains new or old values or both."),
        }
    }

    pub fn add(&mut self, other: &Self) {
        let other_keys = other.0.keys().cloned().collect::<IndexSet<_>>();
        let keys = self.0.keys().cloned().collect::<IndexSet<_>>();
        let union = keys.union(&other_keys);

        for key in union {
            match (self.diff_by_kind(key), other.diff_by_kind(key)) {
                (None, None) => unreachable!("One key of the union set must be present"),
                (None, Some(other_diff)) => {
                    self.0.insert(key.clone(), other_diff.clone());
                }
                (Some(_), None) => {
                    // Nothing to be done
                }
                (Some(this_diff), Some(other_diff)) => {
                    let new_diff = this_diff.add(other_diff);
                    self.0.insert(key.clone(), new_diff);
                }
            }
        }
    }
}

impl<K> Default for MetricsSummary<K>
where
    K: Hash + Eq,
{
    fn default() -> Self {
        Self(IndexMap::default())
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use std::iter;

    use rstest::rstest;

    use super::*;
    use crate::api::EventKind::{self, *};
    use crate::runner::summary::Diffs;
    use crate::util::EitherOrBoth;

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

    #[rstest]
    #[case::all_zero_int(0, 0, 0.0f64)]
    #[case::lhs_zero_int_one(0, 1, 0.0f64)]
    #[case::lhs_zero_int_two(0, 2, 0.0f64)]
    #[case::one_rhs_zero_int(1, 0, 0.0f64)]
    #[case::two_rhs_zero_int(2, 0, 0.0f64)]
    #[case::all_zero_float(0.0f64, 0.0f64, 0.0f64)]
    #[case::lhs_zero_float_one(0.0f64, 1.0f64, 0.0f64)]
    #[case::lhs_zero_float_two(0.0f64, 2.0f64, 0.0f64)]
    #[case::lhs_zero_float_neg_two(0.0f64, -2.0f64, -0.0f64)]
    #[case::one_rhs_zero_float(1.0f64, 0.0f64, 0.0f64)]
    #[case::two_rhs_zero_float(2.0f64, 0.0f64, 0.0f64)]
    #[case::one_neg_rhs_zero_float(1.0f64, -0.0f64, 0.0f64)]
    #[case::one_one_int(1, 1, 1.0f64)]
    #[case::two_one_int(2, 1, 2.0f64)]
    #[case::one_two_int(1, 2, 0.5f64)]
    #[case::one_float_one(1, 1.0f64, 1.0f64)]
    #[case::float_one_int_one(1.0f64, 1, 1.0f64)]
    #[case::float_one(1.0f64, 1.0f64, 1.0f64)]
    #[case::one_float_two(1, 2.0f64, 0.5f64)]
    #[case::float_one_int_two(1.0f64, 2, 0.5f64)]
    #[case::float_one_two(1.0f64, 2.0f64, 0.5f64)]
    fn test_metric_safe_div<L, R, E>(#[case] lhs: L, #[case] rhs: R, #[case] expected: E)
    where
        L: Into<Metric>,
        R: Into<Metric>,
        E: Into<Metric>,
    {
        let expected = expected.into();

        let lhs = lhs.into();
        let rhs = rhs.into();

        assert_eq!(lhs.div0(rhs), expected);
    }

    #[rstest]
    #[case::zero(0, 0, 0)]
    #[case::one_zero(1, 0, 1)]
    #[case::zero_one(0, 1, 1)]
    #[case::u64_max(0, u64::MAX, u64::MAX)]
    #[case::one_u64_max_saturates(1, u64::MAX, u64::MAX)]
    #[case::one(1, 1, 2)]
    #[case::two_one(2, 1, 3)]
    #[case::one_two(1, 2, 3)]
    #[case::float_one_int_zero(1.0f64, 0, 1.0f64)]
    #[case::int_zero_float_one(0, 1.0f64, 1.0f64)]
    #[case::float_zero(0.0f64, 0.0f64, 0.0f64)]
    #[case::float_one(1.0f64, 1.0f64, 2.0f64)]
    #[case::float_one_two(1.0f64, 2.0f64, 3.0f64)]
    #[case::float_two_one(2.0f64, 1.0f64, 3.0f64)]
    fn test_metric_add_and_add_assign<L, R, E>(#[case] lhs: L, #[case] rhs: R, #[case] expected: E)
    where
        L: Into<Metric>,
        R: Into<Metric>,
        E: Into<Metric>,
    {
        let expected = expected.into();

        let mut lhs = lhs.into();
        let rhs = rhs.into();

        assert_eq!(lhs + rhs, expected);

        lhs += rhs;
        assert_eq!(lhs, expected);
    }

    #[rstest]
    #[case::zero("0", 0)]
    #[case::one("1", 1)]
    #[case::u64_max(&format!("{}", u64::MAX), u64::MAX)]
    #[case::one_below_u64_max(&format!("{}", u64::MAX - 1), u64::MAX - 1)]
    #[case::zero_float("0.0", 0.0f64)]
    #[case::one_float("1.0", 1.0f64)]
    #[case::one_point("1.", 1.0f64)]
    #[case::point_one(".1", 0.1f64)]
    #[case::two_float("2.0", 2.0f64)]
    #[case::neg_one_float("-1.0", -1.0f64)]
    #[case::neg_two_float("-2.0", -2.0f64)]
    #[case::inf("inf", f64::INFINITY)]
    fn test_metric_from_str<E>(#[case] input: &str, #[case] expected: E)
    where
        E: Into<Metric>,
    {
        let expected = expected.into();
        assert_eq!(input.parse::<Metric>().unwrap(), expected);
    }

    #[test]
    fn test_metric_from_str_when_invalid_then_error() {
        let err = "abc".parse::<Metric>().unwrap_err();
        assert_eq!(
            "Invalid metric: invalid float literal".to_owned(),
            err.to_string()
        );
    }

    #[rstest]
    #[case::zero(0, 0, 0)]
    #[case::zero_one(0, 1, 0)]
    #[case::one(1, 1, 1)]
    #[case::one_two(1, 2, 2)]
    #[case::u64_max_one(u64::MAX, 1, u64::MAX)]
    #[case::u64_max_two_saturates(u64::MAX, 2, u64::MAX)]
    #[case::zero_float(0, 0.0f64, 0.0f64)]
    #[case::zero_one_float(0, 1.0f64, 0.0f64)]
    #[case::one_float(1, 1.0f64, 1.0f64)]
    #[case::one_two_float(1, 2.0f64, 2.0f64)]
    #[case::u64_max_two_float(u64::MAX, 2.0f64, 2.0f64 * (u64::MAX as f64))]
    fn test_metric_mul_u64<B, E>(#[case] a: u64, #[case] b: B, #[case] expected: E)
    where
        B: Into<Metric>,
        E: Into<Metric>,
    {
        let expected = expected.into();
        let b = b.into();

        assert_eq!(a * b, expected);
        assert_eq!(b * a, expected);
    }

    #[rstest]
    #[case::zero(0, 0, 0)]
    #[case::one_zero(1, 0, 1)]
    #[case::zero_one_saturates(0, 1, 0)]
    #[case::u64_max_saturates(0, u64::MAX, 0)]
    #[case::one_u64_max_saturates(1, u64::MAX, 0)]
    #[case::u64_max_one(u64::MAX, 1, u64::MAX - 1)]
    #[case::one(1, 1, 0)]
    #[case::two_one(2, 1, 1)]
    #[case::one_two(1, 2, 0)]
    #[case::float_one_int_zero(1.0f64, 0, 1.0f64)]
    #[case::int_zero_float_one(0, 1.0f64, -1.0f64)]
    #[case::float_zero(0.0f64, 0.0f64, 0.0f64)]
    #[case::float_one(1.0f64, 1.0f64, 0.0f64)]
    #[case::float_one_two(1.0f64, 2.0f64, -1.0f64)]
    #[case::float_two_one(2.0f64, 1.0f64, 1.0f64)]
    fn test_metric_sub<L, R, E>(#[case] lhs: L, #[case] rhs: R, #[case] expected: E)
    where
        L: Into<Metric>,
        R: Into<Metric>,
        E: Into<Metric>,
    {
        let expected = expected.into();

        let lhs = lhs.into();
        let rhs = rhs.into();

        assert_eq!(lhs - rhs, expected);
    }

    #[rstest]
    #[case::zero(0, 0, Ordering::Equal)]
    #[case::one_zero(1, 0, Ordering::Greater)]
    #[case::zero_float(0.0f64, 0.0f64, Ordering::Equal)]
    #[case::one_zero_float(1.0f64, 0.0f64, Ordering::Greater)]
    #[case::one_int_zero_float(1, 0.0f64, Ordering::Greater)]
    #[case::one_float_zero_int(1.0f64, 0, Ordering::Greater)]
    #[case::some_number(220, 220.0f64, Ordering::Equal)]
    fn test_metric_ordering<L, R>(#[case] lhs: L, #[case] rhs: R, #[case] expected: Ordering)
    where
        L: Into<Metric>,
        R: Into<Metric>,
    {
        let lhs: Metric = lhs.into();
        let rhs = rhs.into();

        assert_eq!(lhs.cmp(&rhs), expected);
        assert_eq!(rhs.cmp(&lhs), expected.reverse());
    }

    fn expected_metrics_diff<D>(metrics: EitherOrBoth<Metric>, diffs: D) -> MetricsDiff
    where
        D: Into<Option<(f64, f64)>>,
    {
        MetricsDiff {
            metrics,
            diffs: diffs
                .into()
                .map(|(diff_pct, factor)| Diffs { diff_pct, factor }),
        }
    }

    fn metrics_fixture(metrics: &[u64]) -> Metrics<EventKind> {
        // events: Ir Dr Dw I1mr D1mr D1mw ILmr DLmr DLmw
        let event_kinds = [
            Ir,
            Dr,
            Dw,
            I1mr,
            D1mr,
            D1mw,
            ILmr,
            DLmr,
            DLmw,
            L1hits,
            LLhits,
            RamHits,
            TotalRW,
            EstimatedCycles,
        ];

        Metrics::with_metric_kinds(
            event_kinds
                .iter()
                .zip(metrics.iter())
                .map(|(e, v)| (*e, *v)),
        )
    }

    fn metrics_summary_fixture<T, U>(kinds: U) -> MetricsSummary<EventKind>
    where
        T: Into<Option<(f64, f64)>> + Clone,
        U: IntoIterator<Item = (EitherOrBoth<Metric>, T)>,
    {
        // events: Ir Dr Dw I1mr D1mr D1mw ILmr DLmr DLmw
        let event_kinds = [
            Ir,
            Dr,
            Dw,
            I1mr,
            D1mr,
            D1mw,
            ILmr,
            DLmr,
            DLmw,
            L1hits,
            LLhits,
            RamHits,
            TotalRW,
            EstimatedCycles,
        ];

        let map: IndexMap<EventKind, MetricsDiff> = event_kinds
            .iter()
            .zip(kinds)
            .map(|(e, (m, d))| (*e, expected_metrics_diff(m.clone(), d.clone())))
            .collect();

        MetricsSummary(map)
    }

    #[rstest]
    #[case::new_zero(EitherOrBoth::Left(0), None)]
    #[case::new_one(EitherOrBoth::Left(1), None)]
    #[case::new_u64_max(EitherOrBoth::Left(u64::MAX), None)]
    #[case::old_zero(EitherOrBoth::Right(0), None)]
    #[case::old_one(EitherOrBoth::Right(1), None)]
    #[case::old_u64_max(EitherOrBoth::Right(u64::MAX), None)]
    #[case::both_zero(
        EitherOrBoth::Both(0, 0),
        (0f64, 1f64)
    )]
    #[case::both_one(
        EitherOrBoth::Both(1, 1),
        (0f64, 1f64)
    )]
    #[case::both_u64_max(
        EitherOrBoth::Both(u64::MAX, u64::MAX),
        (0f64, 1f64)
    )]
    #[case::new_one_old_zero(
        EitherOrBoth::Both(1, 0),
        (f64::INFINITY, f64::INFINITY)
    )]
    #[case::new_one_old_two(
        EitherOrBoth::Both(1, 2),
        (-50f64, -2f64)
    )]
    #[case::new_zero_old_one(
        EitherOrBoth::Both(0, 1),
        (-100f64, f64::NEG_INFINITY)
    )]
    #[case::new_two_old_one(
        EitherOrBoth::Both(2, 1),
        (100f64, 2f64)
    )]
    fn test_metrics_diff_new<T>(#[case] metrics: EitherOrBoth<u64>, #[case] expected_diffs: T)
    where
        T: Into<Option<(f64, f64)>>,
    {
        let expected = expected_metrics_diff(metrics.clone().map(Metric::Int), expected_diffs);
        let actual = MetricsDiff::new(metrics.map(Metric::Int));

        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case::new_new(EitherOrBoth::Left(1), EitherOrBoth::Left(2), EitherOrBoth::Left(3))]
    #[case::new_old(
        EitherOrBoth::Left(1),
        EitherOrBoth::Right(2),
        EitherOrBoth::Both(1, 2)
    )]
    #[case::new_both(
        EitherOrBoth::Left(1),
        EitherOrBoth::Both(2, 5),
        EitherOrBoth::Both(3, 5)
    )]
    #[case::old_old(EitherOrBoth::Right(1), EitherOrBoth::Right(2), EitherOrBoth::Right(3))]
    #[case::old_new(
        EitherOrBoth::Right(1),
        EitherOrBoth::Left(2),
        EitherOrBoth::Both(2, 1)
    )]
    #[case::old_both(
        EitherOrBoth::Right(1),
        EitherOrBoth::Both(2, 5),
        EitherOrBoth::Both(2, 6)
    )]
    #[case::both_new(
        EitherOrBoth::Both(2, 5),
        EitherOrBoth::Left(1),
        EitherOrBoth::Both(3, 5)
    )]
    #[case::both_old(
        EitherOrBoth::Both(2, 5),
        EitherOrBoth::Right(1),
        EitherOrBoth::Both(2, 6)
    )]
    #[case::both_both(
        EitherOrBoth::Both(2, 5),
        EitherOrBoth::Both(1, 3),
        EitherOrBoth::Both(3, 8)
    )]
    #[case::saturating_new(
        EitherOrBoth::Left(u64::MAX),
        EitherOrBoth::Left(1),
        EitherOrBoth::Left(u64::MAX)
    )]
    #[case::saturating_new_other(
        EitherOrBoth::Left(1),
        EitherOrBoth::Left(u64::MAX),
        EitherOrBoth::Left(u64::MAX)
    )]
    #[case::saturating_old(
        EitherOrBoth::Right(u64::MAX),
        EitherOrBoth::Right(1),
        EitherOrBoth::Right(u64::MAX)
    )]
    #[case::saturating_old_other(
        EitherOrBoth::Right(1),
        EitherOrBoth::Right(u64::MAX),
        EitherOrBoth::Right(u64::MAX)
    )]
    #[case::saturating_both(
        EitherOrBoth::Both(u64::MAX, u64::MAX),
        EitherOrBoth::Both(1, 1),
        EitherOrBoth::Both(u64::MAX, u64::MAX)
    )]
    #[case::saturating_both_other(
        EitherOrBoth::Both(1, 1),
        EitherOrBoth::Both(u64::MAX, u64::MAX),
        EitherOrBoth::Both(u64::MAX, u64::MAX)
    )]
    fn test_metrics_diff_add(
        #[case] metric: EitherOrBoth<u64>,
        #[case] other_metric: EitherOrBoth<u64>,
        #[case] expected: EitherOrBoth<u64>,
    ) {
        let new_diff = MetricsDiff::new(metric.map(Metric::Int));
        let old_diff = MetricsDiff::new(other_metric.map(Metric::Int));
        let expected = MetricsDiff::new(expected.map(Metric::Int));

        assert_eq!(new_diff.add(&old_diff), expected);
        assert_eq!(old_diff.add(&new_diff), expected);
    }

    #[rstest]
    #[case::new_ir(&[0], &[], &[(EitherOrBoth::Left(0), None)])]
    #[case::new_is_summarized(&[10, 20, 30, 1, 2, 3, 4, 2, 0], &[],
        &[
            (EitherOrBoth::Left(10), None),
            (EitherOrBoth::Left(20), None),
            (EitherOrBoth::Left(30), None),
            (EitherOrBoth::Left(1), None),
            (EitherOrBoth::Left(2), None),
            (EitherOrBoth::Left(3), None),
            (EitherOrBoth::Left(4), None),
            (EitherOrBoth::Left(2), None),
            (EitherOrBoth::Left(0), None),
            (EitherOrBoth::Left(54), None),
            (EitherOrBoth::Left(0), None),
            (EitherOrBoth::Left(6), None),
            (EitherOrBoth::Left(60), None),
            (EitherOrBoth::Left(264), None),
        ]
    )]
    #[case::old_ir(&[], &[0], &[(EitherOrBoth::Right(0), None)])]
    #[case::old_is_summarized(&[], &[5, 10, 15, 1, 2, 3, 4, 1, 0],
        &[
            (EitherOrBoth::Right(5), None),
            (EitherOrBoth::Right(10), None),
            (EitherOrBoth::Right(15), None),
            (EitherOrBoth::Right(1), None),
            (EitherOrBoth::Right(2), None),
            (EitherOrBoth::Right(3), None),
            (EitherOrBoth::Right(4), None),
            (EitherOrBoth::Right(1), None),
            (EitherOrBoth::Right(0), None),
            (EitherOrBoth::Right(24), None),
            (EitherOrBoth::Right(1), None),
            (EitherOrBoth::Right(5), None),
            (EitherOrBoth::Right(30), None),
            (EitherOrBoth::Right(204), None),
        ]
    )]
    #[case::new_and_old_ir_zero(&[0], &[0], &[(EitherOrBoth::Both(0, 0), (0f64, 1f64))])]
    #[case::new_and_old_summarized_when_equal(
        &[10, 20, 30, 1, 2, 3, 4, 2, 0],
        &[10, 20, 30, 1, 2, 3, 4, 2, 0],
        &[
            (EitherOrBoth::Both(10, 10), (0f64, 1f64)),
            (EitherOrBoth::Both(20, 20), (0f64, 1f64)),
            (EitherOrBoth::Both(30, 30), (0f64, 1f64)),
            (EitherOrBoth::Both(1, 1), (0f64, 1f64)),
            (EitherOrBoth::Both(2, 2), (0f64, 1f64)),
            (EitherOrBoth::Both(3, 3), (0f64, 1f64)),
            (EitherOrBoth::Both(4, 4), (0f64, 1f64)),
            (EitherOrBoth::Both(2, 2), (0f64, 1f64)),
            (EitherOrBoth::Both(0, 0), (0f64, 1f64)),
            (EitherOrBoth::Both(54, 54), (0f64, 1f64)),
            (EitherOrBoth::Both(0, 0), (0f64, 1f64)),
            (EitherOrBoth::Both(6, 6), (0f64, 1f64)),
            (EitherOrBoth::Both(60, 60), (0f64, 1f64)),
            (EitherOrBoth::Both(264, 264), (0f64, 1f64)),
        ]
    )]
    #[case::new_and_old_summarized_when_not_equal(
        &[10, 20, 30, 1, 2, 3, 4, 2, 0],
        &[5, 10, 15, 1, 2, 3, 4, 1, 0],
        &[
            (EitherOrBoth::Both(10, 5), (100f64, 2f64)),
            (EitherOrBoth::Both(20, 10), (100f64, 2f64)),
            (EitherOrBoth::Both(30, 15), (100f64, 2f64)),
            (EitherOrBoth::Both(1, 1), (0f64, 1f64)),
            (EitherOrBoth::Both(2, 2), (0f64, 1f64)),
            (EitherOrBoth::Both(3, 3), (0f64, 1f64)),
            (EitherOrBoth::Both(4, 4), (0f64, 1f64)),
            (EitherOrBoth::Both(2, 1), (100f64, 2f64)),
            (EitherOrBoth::Both(0, 0), (0f64, 1f64)),
            (EitherOrBoth::Both(54, 24), (125f64, 2.25f64)),
            (EitherOrBoth::Both(0, 1), (-100f64, f64::NEG_INFINITY)),
            (EitherOrBoth::Both(6, 5), (20f64, 1.2f64)),
            (EitherOrBoth::Both(60, 30), (100f64, 2f64)),
            (EitherOrBoth::Both(264, 204),
                (29.411_764_705_882_355_f64, 1.294_117_647_058_823_6_f64)
            ),
        ]
    )]
    fn test_metrics_summary_new<V>(
        #[case] new_metrics: &[u64],
        #[case] old_metrics: &[u64],
        #[case] expected: &[(EitherOrBoth<u64>, V)],
    ) where
        V: Into<Option<(f64, f64)>> + Clone,
    {
        use crate::util::EitherOrBoth;

        let expected_metrics_summary = metrics_summary_fixture(
            expected
                .iter()
                .map(|(e, v)| (e.clone().map(Metric::Int), v.clone())),
        );
        let actual = match (
            (!new_metrics.is_empty()).then_some(new_metrics),
            (!old_metrics.is_empty()).then_some(old_metrics),
        ) {
            (None, None) => unreachable!(),
            (Some(new), None) => MetricsSummary::new(EitherOrBoth::Left(metrics_fixture(new))),
            (None, Some(old)) => MetricsSummary::new(EitherOrBoth::Right(metrics_fixture(old))),
            (Some(new), Some(old)) => MetricsSummary::new(EitherOrBoth::Both(
                metrics_fixture(new),
                metrics_fixture(old),
            )),
        };

        assert_eq!(actual, expected_metrics_summary);
    }
}
