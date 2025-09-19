//! The module containing all elements and logic around the [`Metrics`], [`MetricsDiff`], ...

#![allow(clippy::cast_precision_loss)]

use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::{Add, AddAssign, Div, Mul, Sub};
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use either_or_both::EitherOrBoth;
use indexmap::IndexMap;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::summary::Diffs;
use crate::api::{self, CachegrindMetric, DhatMetric, ErrorMetric, EventKind};
use crate::util::{to_string_unsigned_short, Union};

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
    /// An integer `Metric`
    Int(u64),
    /// A float `Metric`
    Float(f64),
}

/// The different metrics distinguished by tool and if it is an error checking tool as `ErrorMetric`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum MetricKind {
    /// The `None` kind if there are no metrics for a tool
    None,
    /// The Callgrind metric kind
    Callgrind(EventKind),
    /// The Cachegrind metric kind
    Cachegrind(CachegrindMetric),
    /// The DHAT metric kind
    Dhat(DhatMetric),
    /// The Memcheck metric kind
    Memcheck(ErrorMetric),
    /// The Helgrind metric kind
    Helgrind(ErrorMetric),
    /// The DRD metric kind
    DRD(ErrorMetric),
}

/// The `Metrics` backed by an [`indexmap::IndexMap`]
///
/// The insertion order is preserved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// If both metrics are present there is also a `Diffs` present
    pub diffs: Option<Diffs>,
    /// Either the `new`, `old` or both metrics
    pub metrics: EitherOrBoth<Metric>,
}

/// The `MetricsSummary` contains all differences between two tool run segments
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct MetricsSummary<K: Hash + Eq = EventKind>(IndexMap<K, MetricsDiff>);

/// Trait for tools which summarize and calculate derived metrics
pub trait Summarize: Hash + Eq + Clone {
    /// Calculate the derived metrics if any
    fn summarize(_: &mut Cow<Metrics<Self>>) {}
}

/// Trait for checking the [`Metric`] type of a metric kind (like [`api::EventKind`])
pub trait TypeChecker {
    /// Return true if the metric kind is a [`Metric::Float`]
    fn is_float(&self) -> bool;
    /// Return true if the metric kind is a [`Metric::Int`]
    fn is_int(&self) -> bool;
    /// Return true if the `Metric` has the expected metric type
    fn verify_metric(&self, metric: Metric) -> bool {
        (self.is_int() && metric.is_int()) || (self.is_float() && metric.is_float())
    }
}

impl Metric {
    /// Divide by `rhs` normally but if rhs is `0` the result is by convention `0.0`
    ///
    /// No difference is made between negative 0.0 and positive 0.0 os rhs value. The result is
    /// always positive 0.0.
    #[must_use]
    // These allow rules are due to the msrv of 1.74.1 which prints a warning. In later versions the
    // floating point comparison in the match arm are fine.
    #[allow(renamed_and_removed_lints)]
    #[allow(illegal_floating_point_literal_pattern)]
    pub fn div0(self, rhs: Self) -> Self {
        match (self, rhs) {
            (_, Self::Int(0) | Self::Float(0.0f64)) => Self::Float(0.0f64),
            (a, b) => a / b,
        }
    }

    /// Return true if this `Metric` is [`Metric::Int`]
    pub fn is_int(&self) -> bool {
        match self {
            Self::Int(_) => true,
            Self::Float(_) => false,
        }
    }

    /// Return true if this `Metric` is [`Metric::Float`]
    pub fn is_float(&self) -> bool {
        match self {
            Self::Int(_) => false,
            Self::Float(_) => true,
        }
    }

    /// If needed and possible convert this metric to the other [`Metric`] returning the result
    ///
    /// A metric is converted if the expected type of the `metric_kind` is [`Metric::Float`] but the
    /// given metric was [`Metric::Int`]. The metrics of float type are usually percentages with a
    /// value range of `0.0` to `100.0`. Converting `u64` to `f64` within this range happens without
    /// precision loss.
    pub fn try_convert<T: Display + TypeChecker>(&self, metric_kind: T) -> Option<(T, Self)> {
        if metric_kind.verify_metric(*self) {
            Some((metric_kind, *self))
        } else if let Self::Int(a) = self {
            Some((metric_kind, Self::Float(*a as f64)))
        } else {
            None
        }
    }
}

impl Add for Metric {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Int(a), Self::Int(b)) => Self::Int(a.saturating_add(b)),
            (Self::Int(a), Self::Float(b)) => Self::Float((a as f64) + b),
            (Self::Float(a), Self::Int(b)) => Self::Float((b as f64) + a),
            (Self::Float(a), Self::Float(b)) => Self::Float(a + b),
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
            Self::Int(a) => f.pad(&format!("{a}")),
            Self::Float(a) => f.pad(&to_string_unsigned_short(*a)),
        }
    }
}

impl Div for Metric {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Int(a), Self::Int(b)) => Self::Float((a as f64) / (b as f64)),
            (Self::Int(a), Self::Float(b)) => Self::Float((a as f64) / b),
            (Self::Float(a), Self::Int(b)) => Self::Float(a / (b as f64)),
            (Self::Float(a), Self::Float(b)) => Self::Float(a / b),
        }
    }
}

impl Eq for Metric {}

impl From<u64> for Metric {
    fn from(value: u64) -> Self {
        Self::Int(value)
    }
}

impl From<f64> for Metric {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<api::Limit> for Metric {
    fn from(value: api::Limit) -> Self {
        match value {
            api::Limit::Int(a) => Self::Int(a),
            api::Limit::Float(f) => Self::Float(f),
        }
    }
}

impl FromStr for Metric {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.parse::<u64>() {
            Ok(a) => Ok(Self::Int(a)),
            Err(_) => match s.parse::<f64>() {
                Ok(a) => Ok(Self::Float(a)),
                Err(error) => Err(anyhow!("Invalid metric: {error}")),
            },
        }
    }
}

impl Mul<u64> for Metric {
    type Output = Self;

    fn mul(self, rhs: u64) -> Self::Output {
        match self {
            Self::Int(a) => Self::Int(a.saturating_mul(rhs)),
            Self::Float(a) => Self::Float(a * (rhs as f64)),
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
            (Self::Int(a), Self::Int(b)) => Self::Int(a.saturating_sub(b)),
            (Self::Int(a), Self::Float(b)) => Self::Float((a as f64) - b),
            (Self::Float(a), Self::Int(b)) => Self::Float(a - (b as f64)),
            (Self::Float(a), Self::Float(b)) => Self::Float(a - b),
        }
    }
}

impl Display for MetricKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => Ok(()),
            Self::Callgrind(metric) => f.write_fmt(format_args!("Callgrind: {metric}")),
            Self::Cachegrind(metric) => f.write_fmt(format_args!("Cachegrind: {metric}")),
            Self::Dhat(metric) => f.write_fmt(format_args!("DHAT: {metric}")),
            Self::Memcheck(metric) => f.write_fmt(format_args!("Memcheck: {metric}")),
            Self::Helgrind(metric) => f.write_fmt(format_args!("Helgrind: {metric}")),
            Self::DRD(metric) => f.write_fmt(format_args!("DRD: {metric}")),
        }
    }
}

impl<K> Metrics<K>
where
    K: Hash + Eq + Display + Clone,
{
    /// Return empty `Metrics`
    pub fn empty() -> Self {
        Self(IndexMap::new())
    }

    /// The order matters. The index is derived from the insertion order
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

    /// Return the contained metric kinds
    pub fn metric_kinds(&self) -> Vec<K> {
        self.0.iter().map(|(k, _)| k.clone()).collect()
    }

    /// Create the union map over this and another `Metrics`
    ///
    /// The order of the keys and their values is preserved. New keys from the `other` Metrics are
    /// appended in their original order.
    pub fn union<'a>(&'a self, other: &'a Self) -> Union<'a, K, Metric> {
        Union::new(&self.0, &other.0)
    }

    /// Return an iterator over the metrics in insertion order
    pub fn iter(&self) -> indexmap::map::Iter<'_, K, Metric> {
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

impl<K> IntoIterator for Metrics<K>
where
    K: Hash + Eq,
{
    type Item = (K, Metric);
    type IntoIter = indexmap::map::IntoIter<K, Metric>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, K> IntoIterator for &'a Metrics<K>
where
    K: Hash + Eq,
{
    type Item = (&'a K, &'a Metric);
    type IntoIter = indexmap::map::Iter<'a, K, Metric>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<I, K> FromIterator<I> for Metrics<K>
where
    K: Hash + Eq + From<I>,
{
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
    /// Create a new `MetricsDiff` from an [`EitherOrBoth<Metric>`][Metric]
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

    /// Sum this metrics diff with another [`MetricsDiff`]
    #[must_use]
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
        let summarized = metrics
            .inspect(|metrics| assert!(!metrics.is_empty()))
            .map(|metrics| {
                let mut summarized = Cow::Owned(metrics);
                K::summarize(&mut summarized);
                summarized
            });

        let diffs = match summarized {
            EitherOrBoth::Left(new) => new
                .into_owned()
                .into_iter()
                .map(|(metric_kind, metric)| {
                    (metric_kind, MetricsDiff::new(EitherOrBoth::Left(metric)))
                })
                .collect(),
            EitherOrBoth::Right(old) => old
                .into_owned()
                .into_iter()
                .map(|(metric_kind, metric)| {
                    (metric_kind, MetricsDiff::new(EitherOrBoth::Right(metric)))
                })
                .collect(),
            EitherOrBoth::Both(new, old) => new
                .union(&old)
                .into_iter()
                .map(|(metric_kind, metric)| (metric_kind, MetricsDiff::new(metric)))
                .collect(),
        };

        Self(diffs)
    }

    /// Try to return a [`MetricsDiff`] for the specified `MetricKind`
    pub fn diff_by_kind(&self, metric_kind: &K) -> Option<&MetricsDiff> {
        self.0.get(metric_kind)
    }

    /// Return an iterator over all [`MetricsDiff`]s
    pub fn all_diffs(&self) -> impl Iterator<Item = (&K, &MetricsDiff)> {
        self.0.iter()
    }

    /// Extract the [`Metrics`] from this summary
    ///
    /// This is the exact reverse operation to [`MetricsSummary::new`]
    pub fn extract_costs(&self) -> EitherOrBoth<Metrics<K>> {
        self.0
            .iter()
            .map(|(metric_kind, diff)| diff.metrics.map(|metric| (metric_kind.clone(), metric)))
            .collect::<EitherOrBoth<IndexMap<_, _>>>()
            .map(Metrics)
    }

    /// Sum up another `MetricsSummary` with this one
    ///
    /// If a [`MetricsDiff`] is not present in this summary but in the other, it is added to this
    /// summary.
    pub fn add(&mut self, other: &Self) {
        for (other_key, other_value) in &other.0 {
            if let Some(value) = self.0.get_mut(other_key) {
                *value = value.add(other_value);
            } else {
                self.0.insert(other_key.clone(), other_value.clone());
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

impl From<Metric> for f64 {
    fn from(value: Metric) -> Self {
        match value {
            Metric::Int(a) => a as Self,
            Metric::Float(a) => a,
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

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use std::{f64, iter};

    use either_or_both::EitherOrBoth;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;
    use crate::api::EventKind::{self, *};
    use crate::runner::summary::Diffs;

    fn expected_metrics<I, T>(events: T) -> Metrics<EventKind>
    where
        I: Into<Metric>,
        T: IntoIterator<Item = (EventKind, I)>,
    {
        Metrics(
            events
                .into_iter()
                .map(|(k, n)| (k, n.into()))
                .collect::<IndexMap<_, _>>(),
        )
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
            I1MissRate,
            D1MissRate,
            LLiMissRate,
            LLdMissRate,
            LLMissRate,
            L1HitRate,
            LLHitRate,
            RamHitRate,
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
            I1MissRate,
            D1MissRate,
            LLiMissRate,
            LLdMissRate,
            LLMissRate,
            L1HitRate,
            LLHitRate,
            RamHitRate,
        ];

        let map: IndexMap<EventKind, MetricsDiff> = event_kinds
            .iter()
            .zip(kinds)
            .map(|(e, (m, d))| (*e, expected_metrics_diff(m, d)))
            .collect();

        MetricsSummary(map)
    }

    #[rstest]
    #[case::single_zero(&[Ir], &["0"], expected_metrics([(Ir, 0)]))]
    #[case::single_one(&[Ir], &["1"], expected_metrics([(Ir, 1)]))]
    #[case::single_float(&[Ir], &["1.0"], expected_metrics([(Ir, 1.0f64)]))]
    #[case::single_u64_max(&[Ir], &[u64::MAX.to_string()], expected_metrics([(Ir, u64::MAX)]))]
    #[case::one_more_than_max_u64(&[Ir], &["18446744073709551616"],
        // This float has the correct value to represent the value above
        expected_metrics([(Ir, 18_446_744_073_709_552_000_f64)])
    )]
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
    #[case::word(&[Ir], &["abc"])]
    #[case::empty(&[Ir], &[""])]
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
        let expected = expected_metrics_diff(metrics.map(Metric::Int), expected_diffs);
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
    #[case::new_ir(&[0], &[], &[(EitherOrBoth::Left(Metric::Int(0)), None)])]
    #[case::new_is_summarized(&[10, 20, 30, 1, 2, 3, 4, 2, 0], &[],
        &[
            (EitherOrBoth::Left(Metric::Int(10)), None),
            (EitherOrBoth::Left(Metric::Int(20)), None),
            (EitherOrBoth::Left(Metric::Int(30)), None),
            (EitherOrBoth::Left(Metric::Int(1)), None),
            (EitherOrBoth::Left(Metric::Int(2)), None),
            (EitherOrBoth::Left(Metric::Int(3)), None),
            (EitherOrBoth::Left(Metric::Int(4)), None),
            (EitherOrBoth::Left(Metric::Int(2)), None),
            (EitherOrBoth::Left(Metric::Int(0)), None),
            (EitherOrBoth::Left(Metric::Int(54)), None),
            (EitherOrBoth::Left(Metric::Int(0)), None),
            (EitherOrBoth::Left(Metric::Int(6)), None),
            (EitherOrBoth::Left(Metric::Int(60)), None),
            (EitherOrBoth::Left(Metric::Int(264)), None),
            (EitherOrBoth::Left(Metric::Float(10f64)), None),
            (EitherOrBoth::Left(Metric::Float(10f64)), None),
            (EitherOrBoth::Left(Metric::Float(40f64)), None),
            (EitherOrBoth::Left(Metric::Float(4f64)), None),
            (EitherOrBoth::Left(Metric::Float(10f64)), None),
            (EitherOrBoth::Left(Metric::Float(90f64)), None),
            (EitherOrBoth::Left(Metric::Float(0f64)), None),
            (EitherOrBoth::Left(Metric::Float(10f64)), None),
        ]
    )]
    #[case::old_ir(&[], &[0], &[(EitherOrBoth::Right(Metric::Int(0)), None)])]
    #[case::old_is_summarized(&[], &[5, 10, 15, 1, 2, 3, 4, 1, 0],
        &[
            (EitherOrBoth::Right(Metric::Int(5)), None),
            (EitherOrBoth::Right(Metric::Int(10)), None),
            (EitherOrBoth::Right(Metric::Int(15)), None),
            (EitherOrBoth::Right(Metric::Int(1)), None),
            (EitherOrBoth::Right(Metric::Int(2)), None),
            (EitherOrBoth::Right(Metric::Int(3)), None),
            (EitherOrBoth::Right(Metric::Int(4)), None),
            (EitherOrBoth::Right(Metric::Int(1)), None),
            (EitherOrBoth::Right(Metric::Int(0)), None),
            (EitherOrBoth::Right(Metric::Int(24)), None),
            (EitherOrBoth::Right(Metric::Int(1)), None),
            (EitherOrBoth::Right(Metric::Int(5)), None),
            (EitherOrBoth::Right(Metric::Int(30)), None),
            (EitherOrBoth::Right(Metric::Int(204)), None),
            (EitherOrBoth::Right(Metric::Float(20f64)), None),
            (EitherOrBoth::Right(Metric::Float(20f64)), None),
            (EitherOrBoth::Right(Metric::Float(80f64)), None),
            (EitherOrBoth::Right(Metric::Float(4f64)), None),
            (EitherOrBoth::Right(Metric::Float(16.666_666_666_666_664_f64)), None),
            (EitherOrBoth::Right(Metric::Float(80f64)), None),
            (EitherOrBoth::Right(Metric::Float(3.333_333_333_333_333_5_f64)), None),
            (EitherOrBoth::Right(Metric::Float(16.666_666_666_666_664_f64)), None),
        ]
    )]
    #[case::new_and_old_ir_zero(&[0], &[0], &[
        (EitherOrBoth::Both(Metric::Int(0), Metric::Int(0)), (0f64, 1f64))
    ])]
    #[case::new_and_old_summarized_when_equal(
        &[10, 20, 30, 1, 2, 3, 4, 2, 0],
        &[10, 20, 30, 1, 2, 3, 4, 2, 0],
        &[
            (EitherOrBoth::Both(Metric::Int(10), Metric::Int(10)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(20), Metric::Int(20)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(30), Metric::Int(30)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(1), Metric::Int(1)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(2), Metric::Int(2)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(3), Metric::Int(3)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(4), Metric::Int(4)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(2), Metric::Int(2)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(0), Metric::Int(0)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(54), Metric::Int(54)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(0), Metric::Int(0)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(6), Metric::Int(6)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(60), Metric::Int(60)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(264), Metric::Int(264)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Float(10f64), Metric::Float(10f64)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Float(10f64), Metric::Float(10f64)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Float(40f64), Metric::Float(40f64)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Float(4f64), Metric::Float(4f64)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Float(10f64), Metric::Float(10f64)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Float(90f64), Metric::Float(90f64)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Float(0f64), Metric::Float(0f64)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Float(10f64), Metric::Float(10f64)), (0f64, 1f64)),
        ]
    )]
    #[case::new_and_old_summarized_when_not_equal(
        &[10, 20, 30, 1, 2, 3, 4, 2, 0],
        &[5, 10, 15, 1, 2, 3, 4, 1, 0],
        &[
            (EitherOrBoth::Both(Metric::Int(10), Metric::Int(5)), (100f64, 2f64)),
            (EitherOrBoth::Both(Metric::Int(20), Metric::Int(10)), (100f64, 2f64)),
            (EitherOrBoth::Both(Metric::Int(30), Metric::Int(15)), (100f64, 2f64)),
            (EitherOrBoth::Both(Metric::Int(1), Metric::Int(1)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(2), Metric::Int(2)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(3), Metric::Int(3)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(4), Metric::Int(4)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(2), Metric::Int(1)), (100f64, 2f64)),
            (EitherOrBoth::Both(Metric::Int(0), Metric::Int(0)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Int(54), Metric::Int(24)), (125f64, 2.25f64)),
            (EitherOrBoth::Both(Metric::Int(0), Metric::Int(1)), (-100f64, f64::NEG_INFINITY)),
            (EitherOrBoth::Both(Metric::Int(6), Metric::Int(5)), (20f64, 1.2f64)),
            (EitherOrBoth::Both(Metric::Int(60), Metric::Int(30)), (100f64, 2f64)),
            (EitherOrBoth::Both(Metric::Int(264), Metric::Int(204)),
                (29.411_764_705_882_355_f64, 1.294_117_647_058_823_6_f64)
            ),
            (EitherOrBoth::Both(Metric::Float(10f64), Metric::Float(20f64)), (-50f64, -2f64)),
            (EitherOrBoth::Both(Metric::Float(10f64), Metric::Float(20f64)), (-50f64, -2f64)),
            (EitherOrBoth::Both(Metric::Float(40f64), Metric::Float(80f64)), (-50f64, -2f64)),
            (EitherOrBoth::Both(Metric::Float(4f64), Metric::Float(4f64)), (0f64, 1f64)),
            (EitherOrBoth::Both(Metric::Float(10f64), Metric::Float(16.666_666_666_666_664_f64)),
                (-39.999_999_999_999_99_f64, -1.666_666_666_666_666_5_f64)
            ),
            (EitherOrBoth::Both(Metric::Float(90f64), Metric::Float(80f64)), (12.5f64, 1.125f64)),
            (EitherOrBoth::Both(Metric::Float(0f64), Metric::Float(3.333_333_333_333_333_5_f64)),
                (-100f64, f64::NEG_INFINITY)
            ),
            (EitherOrBoth::Both(Metric::Float(10f64), Metric::Float(16.666_666_666_666_664_f64)),
                (-39.999_999_999_999_99_f64, -1.666_666_666_666_666_5_f64)
            ),
        ]
    )]
    fn test_metrics_summary_new<V>(
        #[case] new_metrics: &[u64],
        #[case] old_metrics: &[u64],
        #[case] expected: &[(EitherOrBoth<Metric>, V)],
    ) where
        V: Into<Option<(f64, f64)>> + Clone,
    {
        use either_or_both::EitherOrBoth;

        let expected_metrics_summary =
            metrics_summary_fixture(expected.iter().map(|(e, v)| (*e, v.clone())));
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
