use std::borrow::Cow;
use std::fmt::{Debug, Display};
use std::fs::File;
use std::hash::Hash;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use derive_more::AsRef;
use glob::glob;
use indexmap::{indexmap, IndexMap, IndexSet};
use itertools::Itertools;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::{Baselines, ModulePath};
use super::format::{Formatter, OutputFormat, OutputFormatKind, VerticalFormatter};
use super::metrics::Metrics;
use super::tool::parser::ParserOutput;
use crate::api::{CachegrindMetric, DhatMetric, ErrorMetric, EventKind, ValgrindTool};
use crate::error::Error;
use crate::runner::metrics::Summarize;
use crate::util::{factor_diff, make_absolute, percentage_diff, EitherOrBoth};

pub type RegressionMetrics<T> = (T, u64, u64, f64, f64);

/// A `Baseline` depending on the [`BaselineKind`] which points to the corresponding path
///
/// This baseline is used for comparisons with the new output of valgrind tools.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Baseline {
    /// The kind of the `Baseline`
    pub kind: BaselineKind,
    /// The path to the file which is used to compare against the new output
    pub path: PathBuf,
}

/// The `BaselineKind` describing the baseline
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum BaselineKind {
    /// Compare new against `*.old` output files
    Old,
    /// Compare new against a named baseline
    Name(BaselineName),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct BaselineName(String);

/// The `BenchmarkKind`, differentiating between library and binary benchmarks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum BenchmarkKind {
    /// A library benchmark
    LibraryBenchmark,
    /// A binary benchmark
    BinaryBenchmark,
}

/// The `BenchmarkSummary` containing all the information of a single benchmark run
///
/// This includes produced files, recorded callgrind events, performance regressions ...
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct BenchmarkSummary {
    /// The version of this format. Only backwards incompatible changes cause an increase of the
    /// version
    pub version: String,
    /// Whether this summary describes a library or binary benchmark
    pub kind: BenchmarkKind,
    /// The destination and kind of the summary file
    pub summary_output: Option<SummaryOutput>,
    /// The project's root directory
    pub project_root: PathBuf,
    /// The directory of the package
    pub package_dir: PathBuf,
    /// The path to the benchmark file
    pub benchmark_file: PathBuf,
    /// The path to the binary which is executed by valgrind. In case of a library benchmark this
    /// is the compiled benchmark file. In case of a binary benchmark this is the path to the
    /// command.
    pub benchmark_exe: PathBuf,
    /// The name of the function under test
    pub function_name: String,
    /// The rust path in the form `bench_file::group::bench`
    pub module_path: String,
    /// The user provided id of this benchmark
    pub id: Option<String>,
    /// More details describing this benchmark run
    pub details: Option<String>,
    /// The baselines if any. An absent first baseline indicates that new output was produced. An
    /// absent second baseline indicates the usage of the usual "*.old" output.
    pub baselines: (Option<String>, Option<String>),
    /// The summary of other valgrind tool runs
    pub profiles: Profiles,
}

/// The differences between two `Metrics` as percentage and factor
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Diffs {
    /// The percentage of the difference between two `Metrics` serialized as string to preserve
    /// infinity values and avoid `null` in json
    #[serde(with = "crate::serde::float_64")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub diff_pct: f64,
    /// The factor of the difference between two `Metrics` serialized as string to preserve
    /// infinity values and void `null` in json
    #[serde(with = "crate::serde::float_64")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub factor: f64,
}

/// All callgrind flamegraph summaries and their totals
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct FlamegraphSummaries {
    /// The `FlamegraphSummary`s
    pub summaries: Vec<FlamegraphSummary>,
    /// The totals over the `FlamegraphSummary`s
    pub totals: Vec<FlamegraphSummary>,
}

/// The callgrind `FlamegraphSummary` records all created paths for an [`EventKind`] specific
/// flamegraph
///
/// Either the `regular_path`, `old_path` or the `diff_path` are present. Never can all of them be
/// absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct FlamegraphSummary {
    /// The `EventKind` of the flamegraph
    pub event_kind: EventKind,
    /// If present, the path to the file of the regular (non-differential) flamegraph
    pub regular_path: Option<PathBuf>,
    /// If present, the path to the file of the old regular (non-differential) flamegraph
    pub base_path: Option<PathBuf>,
    /// If present, the path to the file of the differential flamegraph
    pub diff_path: Option<PathBuf>,
}

/// The different metrics distinguished by tool and if it is an error checking tool as `ErrorMetric`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum MetricKind {
    None,
    Callgrind(EventKind),
    Cachegrind(CachegrindMetric),
    Dhat(DhatMetric),
    ErrorMetric(ErrorMetric),
}

/// The `MetricsDiff` describes the difference between a `new` and `old` metric as percentage and
/// factor.
///
/// Only if both metrics are present there is also a `Diffs` present. Otherwise, it just stores the
/// `new` or `old` metric.
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct MetricsDiff {
    /// Either the `new`, `old` or both metrics
    pub metrics: EitherOrBoth<u64>,
    /// If both metrics are present there is also a `Diffs` present
    pub diffs: Option<Diffs>,
}

/// The `MetricsSummary` contains all differences between two tool run segments
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct MetricsSummary<K: Hash + Eq = EventKind>(IndexMap<K, MetricsDiff>);

/// The `ToolSummary` containing all information about a valgrind tool run
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Profile {
    /// The Valgrind tool like `DHAT`, `Memcheck` etc.
    pub tool: ValgrindTool,
    /// The paths to the `*.log` files. All tools produce at least one log file
    pub log_paths: Vec<PathBuf>,
    /// The paths to the `*.out` files. Not all tools produce an output in addition to the log
    /// files
    pub out_paths: Vec<PathBuf>,
    /// Details and information about the created flamegraphs if any
    pub flamegraphs: Vec<FlamegraphSummary>,
    /// The metrics and details about the tool run
    pub summaries: ProfileData,
}

/// The `ToolRun` contains all information about a single tool run with possibly multiple segments
///
/// The total is always present and summarizes all tool run segments. In the special case of a
/// single tool run segment, the total equals the metrics of this segment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ProfileData {
    /// All [`ProfilePart`]s
    pub parts: Vec<ProfilePart>,
    /// The total over the [`ProfilePart`]s
    pub total: ProfileTotal,
}

/// Some additional and necessary information about the tool run segment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AsRef)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ProfileInfo {
    /// The executed command extracted from Valgrind output
    pub command: String,
    /// The pid of this process
    pub pid: i32,
    /// The parent pid of this process
    pub parent_pid: Option<i32>,
    /// More details for example from the logging output of the tool run
    pub details: Option<String>,
    /// The path to the file from the tool run
    pub path: PathBuf,
    /// The part of this tool run (only callgrind)
    pub part: Option<u64>,
    /// The thread of this tool run (only callgrind)
    pub thread: Option<usize>,
}

/// A single segment of a tool run and if present the comparison with the "old" segment
///
/// A tool run can produce multiple segments, for example for each process and subprocess with
/// (--trace-children).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ProfilePart {
    /// Details like command, pid, ppid, thread number etc. (see [`ProfileInfo`])
    pub details: EitherOrBoth<ProfileInfo>,
    /// The [`ToolMetricSummary`]
    pub metrics_summary: ToolMetricSummary,
}

/// The total metrics over all [`ProfilePart`]s and if detected any [`ToolRegression`]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ProfileTotal {
    pub summary: ToolMetricSummary,
    pub regressions: Vec<ToolRegression>,
}

/// The collection of all generated [`Profile`]s
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Default)]
pub struct Profiles(Vec<Profile>);

/// The format (json, ...) in which the summary file should be saved or printed
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum SummaryFormat {
    /// The format in a space optimal json representation without newlines
    Json,
    /// The format in pretty printed json
    PrettyJson,
}

/// Manage the summary output file with this `SummaryOutput`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct SummaryOutput {
    /// The [`SummaryFormat`]
    format: SummaryFormat,
    /// The path to the destination file of this summary
    path: PathBuf,
}

/// The metrics distinguished per tool class
///
/// The tool classes are: dhat, error metrics from memcheck, drd, helgrind and callgrind
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum ToolMetrics {
    /// If there were no metrics extracted from a tool (currently massif, bbv)
    #[default]
    None,
    /// The metrics of a dhat benchmark
    Dhat(Metrics<DhatMetric>),
    /// The metrics of a tool run which reports errors (memcheck, helgrind, drd)
    ErrorTool(Metrics<ErrorMetric>),
    /// The metrics of a callgrind benchmark
    Callgrind(Metrics<EventKind>),
    /// The metrics of a cachegrind benchmark
    Cachegrind(Metrics<CachegrindMetric>),
}

/// The `ToolMetricSummary` contains the `MetricsSummary` distinguished by tool and metric kinds
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum ToolMetricSummary {
    /// If there are no metrics extracted (currently massif, bbv)
    #[default]
    None,
    /// The error summary of tools which reports errors (memcheck, helgrind, drd)
    ErrorTool(MetricsSummary<ErrorMetric>),
    /// The dhat summary
    Dhat(MetricsSummary<DhatMetric>),
    /// The callgrind summary
    Callgrind(MetricsSummary<EventKind>),
    /// The cachegrind summary
    Cachegrind(MetricsSummary<CachegrindMetric>),
}

// The regression of a specific `MetricKind`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ToolRegression {
    pub metric: MetricKind,
    /// The value of the new benchmark run
    pub new: u64,
    /// The value of the old benchmark run
    pub old: u64,
    /// The difference between new and old in percent. Serialized as string to preserve infinity
    /// values and avoid null in json.
    #[serde(with = "crate::serde::float_64")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub diff_pct: f64,
    /// The value of the limit which was exceeded to cause a performance regression. Serialized as
    /// string to preserve infinity values and avoid null in json.
    #[serde(with = "crate::serde::float_64")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub limit: f64,
}

impl Display for BaselineName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for BaselineName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for char in s.chars() {
            if !(char.is_ascii_alphanumeric() || char == '_') {
                return Err(format!(
                    "A baseline name can only consist of ascii characters which are alphanumeric \
                     or '_' but found: '{char}'"
                ));
            }
        }
        Ok(Self(s.to_owned()))
    }
}

impl BenchmarkSummary {
    /// Create a new `BenchmarkSummary`
    ///
    /// Relative paths are made absolute with the `project_root` as base directory.
    pub fn new(
        kind: BenchmarkKind,
        project_root: PathBuf,
        package_dir: PathBuf,
        benchmark_file: PathBuf,
        benchmark_exe: PathBuf,
        module_path: &ModulePath,
        function_name: &str,
        id: Option<String>,
        details: Option<String>,
        output: Option<SummaryOutput>,
        baselines: Baselines,
    ) -> Self {
        Self {
            version: "3".to_owned(),
            kind,
            benchmark_file: make_absolute(&project_root, benchmark_file),
            benchmark_exe: make_absolute(&project_root, benchmark_exe),
            module_path: module_path.to_string(),
            function_name: function_name.to_owned(),
            id,
            details,
            profiles: Profiles::default(),
            summary_output: output,
            project_root,
            package_dir,
            baselines,
        }
    }

    pub fn print_and_save(&self, output_format: &OutputFormatKind) -> Result<()> {
        let value = match (output_format, &self.summary_output) {
            (OutputFormatKind::Default, None) => return Ok(()),
            _ => {
                serde_json::to_value(self).with_context(|| "Failed to serialize summary to json")?
            }
        };

        let result = match output_format {
            OutputFormatKind::Default => Ok(()),
            OutputFormatKind::Json => {
                let output = stdout();
                let writer = output.lock();
                let result = serde_json::to_writer(writer, &value);
                println!();
                result
            }
            OutputFormatKind::PrettyJson => {
                let output = stdout();
                let writer = output.lock();
                let result = serde_json::to_writer_pretty(writer, &value);
                println!();
                result
            }
        };
        result.with_context(|| "Failed to print json to stdout")?;

        if let Some(output) = &self.summary_output {
            let file = output.create()?;

            let result = if matches!(output.format, SummaryFormat::PrettyJson) {
                serde_json::to_writer_pretty(file, &value)
            } else {
                serde_json::to_writer(file, &value)
            };

            result.with_context(|| {
                format!("Failed to write summary to file: {}", output.path.display())
            })?;
        }

        Ok(())
    }

    /// Check if this `BenchmarkSummary` has recorded any performance regressions
    ///
    /// # Errors
    ///
    /// If a regressions is present and are configured to be `fail_fast` an error is returned
    pub fn check_regression(&self, fail_fast: bool) -> Result<()> {
        if self.profiles.is_regressed() && fail_fast {
            return Err(Error::RegressionError(true).into());
        }

        Ok(())
    }

    /// Return true if any [`Profile`] has regressed
    pub fn is_regressed(&self) -> bool {
        self.profiles.is_regressed()
    }

    pub fn compare_and_print(
        &self,
        id: &str,
        other: &Self,
        output_format: &OutputFormat,
    ) -> Result<()> {
        let mut summaries = vec![];

        for profile in self.profiles.iter() {
            if let Some(other_profile) = other.profiles.iter().find(|s| s.tool == profile.tool) {
                if let Some(summary) = ToolMetricSummary::from_self_and_other(
                    &profile.summaries.total.summary,
                    &other_profile.summaries.total.summary,
                ) {
                    summaries.push((profile.tool, summary));
                }
            }
        }

        // There really should always be at least one summary. Also, if the default tool is massif
        // or bbv which (currently) don't have an actual summary.
        if summaries.is_empty() {
            Ok(())
        } else {
            VerticalFormatter::new(output_format.clone()).print_comparison(
                &self.function_name,
                id,
                self.details.as_deref(),
                summaries,
            )
        }
    }
}

impl Diffs {
    pub fn new(new: u64, old: u64) -> Self {
        Self {
            diff_pct: percentage_diff(new, old),
            factor: factor_diff(new, old),
        }
    }
}

impl FlamegraphSummary {
    /// Create a new `FlamegraphSummary`
    pub fn new(event_kind: EventKind) -> Self {
        Self {
            event_kind,
            regular_path: Option::default(),
            base_path: Option::default(),
            diff_path: Option::default(),
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
            MetricKind::ErrorMetric(metric) => f.write_fmt(format_args!("Error: {metric}")),
        }
    }
}

impl MetricsDiff {
    pub fn new(metrics: EitherOrBoth<u64>) -> Self {
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
                Self::new(EitherOrBoth::Left(new.saturating_add(*other_new)))
            }
            (EitherOrBoth::Right(old), EitherOrBoth::Left(new))
            | (EitherOrBoth::Left(new), EitherOrBoth::Right(old)) => {
                Self::new(EitherOrBoth::Both(*new, *old))
            }
            (EitherOrBoth::Right(old), EitherOrBoth::Right(other_old)) => {
                Self::new(EitherOrBoth::Right(old.saturating_add(*other_old)))
            }
            (EitherOrBoth::Both(new, old), EitherOrBoth::Left(other_new))
            | (EitherOrBoth::Left(new), EitherOrBoth::Both(other_new, old)) => {
                Self::new(EitherOrBoth::Both(new.saturating_add(*other_new), *old))
            }
            (EitherOrBoth::Both(new, old), EitherOrBoth::Right(other_old))
            | (EitherOrBoth::Right(old), EitherOrBoth::Both(new, other_old)) => {
                Self::new(EitherOrBoth::Both(*new, old.saturating_add(*other_old)))
            }
            (EitherOrBoth::Both(new, old), EitherOrBoth::Both(other_new, other_old)) => {
                Self::new(EitherOrBoth::Both(
                    new.saturating_add(*other_new),
                    old.saturating_add(*other_old),
                ))
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

impl Profile {
    pub fn is_regressed(&self) -> bool {
        self.summaries.is_regressed()
    }
}

impl ProfileData {
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    pub fn is_regressed(&self) -> bool {
        self.total.is_regressed()
    }

    pub fn has_multiple(&self) -> bool {
        self.parts.len() > 1
    }

    /// Used internally to group the output by pid, then by parts and then by threads
    ///
    /// The grouping simplifies the zipping of the new and old parser output later.
    ///
    /// A simplified example. `(pid, part, thread)`
    ///
    /// ```rust,ignore
    /// let parsed: Vec<(i32, u64, usize)> = [
    ///     (10, 1, 1),
    ///     (10, 1, 2),
    ///     (20, 1, 1)
    /// ];
    ///
    /// let grouped = group(parsed);
    /// assert_eq!(grouped,
    /// vec![
    ///     vec![
    ///         vec![
    ///             (10, 1, 1),
    ///             (10, 1, 2)
    ///         ]
    ///     ],
    ///     vec![
    ///         vec![
    ///             (20, 1, 1)
    ///         ]
    ///     ]
    /// ])
    /// ```
    fn group(parsed: impl Iterator<Item = ParserOutput>) -> Vec<Vec<Vec<ParserOutput>>> {
        let mut grouped = vec![];
        let mut cur_pid = 0_i32;
        let mut cur_part = 0;

        for element in parsed {
            let pid = element.header.pid;
            let part = element.header.part.unwrap_or(0);

            if pid != cur_pid {
                grouped.push(vec![vec![element]]);
                cur_pid = pid;
                cur_part = part;
            } else if part != cur_part {
                let parts = grouped.last_mut().unwrap();
                parts.push(vec![element]);
                cur_part = part;
            } else {
                let parts = grouped.last_mut().unwrap();
                let threads = parts.last_mut().unwrap();
                threads.push(element);
            }
        }
        grouped
    }

    /// Create a new `ToolRun` from the output(s) of the tool parsers
    ///
    /// The summaries created from the new parser outputs and the old parser outputs are grouped by
    /// pid (subprocesses recorded with `--trace-children`), then by part (for example cause by a
    /// `--dump-every-bb=xxx`) and then by thread (caused by `--separate-threads`). Since each of
    /// these components can differ between the new and the old parser output, this complicates the
    /// creation of each [`ProfileData`]. We can't just zip the new and old parser output directly
    /// to get (as far as possible) correct comparisons between the new and old costs. To remedy
    /// the possibly incorrect comparisons, there is always a total created.
    ///
    /// In a first step the parsed outputs are grouped in vectors by pid, then by parts and then by
    /// threads. This solution is not very efficient but there are not too many parsed outputs to be
    /// expected. 100 at most and maybe 2-10 on average, so the tradeoff between performance and
    /// clearer structure of this method looks reasonable.
    ///
    /// Secondly and finally, the groups are processed and summarized in a total.
    pub fn new(parsed_new: Vec<ParserOutput>, parsed_old: Option<Vec<ParserOutput>>) -> Self {
        let mut total = match parsed_new
            .first()
            .expect("At least 1 parsed result should be present")
            .metrics
        {
            ToolMetrics::None => ToolMetricSummary::None,
            ToolMetrics::Dhat(_) => ToolMetricSummary::Dhat(MetricsSummary::default()),
            ToolMetrics::ErrorTool(_) => ToolMetricSummary::ErrorTool(MetricsSummary::default()),
            ToolMetrics::Callgrind(_) => ToolMetricSummary::Callgrind(MetricsSummary::default()),
            ToolMetrics::Cachegrind(_) => ToolMetricSummary::Cachegrind(MetricsSummary::default()),
        };

        let grouped_new = Self::group(parsed_new.into_iter());
        let grouped_old = Self::group(parsed_old.into_iter().flatten());

        let mut summaries = vec![];

        for e_pids in grouped_new.into_iter().zip_longest(grouped_old) {
            match e_pids {
                itertools::EitherOrBoth::Both(new_parts, old_parts) => {
                    for e_parts in new_parts.into_iter().zip_longest(old_parts) {
                        match e_parts {
                            itertools::EitherOrBoth::Both(new_threads, old_threads) => {
                                for e_threads in new_threads.into_iter().zip_longest(old_threads) {
                                    let summary = match e_threads {
                                        itertools::EitherOrBoth::Both(new, old) => {
                                            ProfilePart::from_new_and_old(new, old)
                                        }
                                        itertools::EitherOrBoth::Left(new) => {
                                            ProfilePart::from_new(new)
                                        }
                                        itertools::EitherOrBoth::Right(old) => {
                                            ProfilePart::from_old(old)
                                        }
                                    };
                                    total.add_mut(&summary.metrics_summary);
                                    summaries.push(summary);
                                }
                            }
                            itertools::EitherOrBoth::Left(left) => {
                                for new in left {
                                    let summary = ProfilePart::from_new(new);
                                    total.add_mut(&summary.metrics_summary);
                                    summaries.push(summary);
                                }
                            }
                            itertools::EitherOrBoth::Right(right) => {
                                for old in right {
                                    let summary = ProfilePart::from_old(old);
                                    total.add_mut(&summary.metrics_summary);
                                    summaries.push(summary);
                                }
                            }
                        }
                    }
                }
                itertools::EitherOrBoth::Left(left) => {
                    for new in left.into_iter().flatten() {
                        let summary = ProfilePart::from_new(new);
                        total.add_mut(&summary.metrics_summary);
                        summaries.push(summary);
                    }
                }
                itertools::EitherOrBoth::Right(right) => {
                    for old in right.into_iter().flatten() {
                        let summary = ProfilePart::from_old(old);
                        total.add_mut(&summary.metrics_summary);
                        summaries.push(summary);
                    }
                }
            }
        }

        Self {
            parts: summaries,
            total: ProfileTotal {
                summary: total,
                regressions: vec![],
            },
        }
    }
}

impl From<ParserOutput> for ProfileInfo {
    fn from(value: ParserOutput) -> Self {
        Self {
            command: value.header.command,
            pid: value.header.pid,
            parent_pid: value.header.parent_pid,
            details: (!value.details.is_empty()).then(|| value.details.join("\n")),
            path: value.path,
            part: value.header.part,
            thread: value.header.thread,
        }
    }
}

impl ProfilePart {
    pub fn new_has_errors(&self) -> bool {
        match &self.metrics_summary {
            ToolMetricSummary::None
            | ToolMetricSummary::Dhat(_)
            | ToolMetricSummary::Cachegrind(_)
            | ToolMetricSummary::Callgrind(_) => false,
            ToolMetricSummary::ErrorTool(metrics) => metrics
                .diff_by_kind(&ErrorMetric::Errors)
                .is_some_and(|e| match e.metrics {
                    EitherOrBoth::Left(new) | EitherOrBoth::Both(new, _) => new > 0,
                    EitherOrBoth::Right(_) => false,
                }),
        }
    }

    pub fn from_new(new: ParserOutput) -> Self {
        let metrics_summary = ToolMetricSummary::from_new_metrics(&new.metrics);
        Self {
            details: EitherOrBoth::Left(new.into()),
            metrics_summary,
        }
    }

    pub fn from_old(old: ParserOutput) -> Self {
        let metrics_summary = ToolMetricSummary::from_old_metrics(&old.metrics);
        Self {
            details: EitherOrBoth::Left(old.into()),
            metrics_summary,
        }
    }

    /// Create a new `ProfilePart` from new and old [`ParserOutput`]
    ///
    /// # Panics
    ///
    /// Treat new and old with different metric kinds as programming error and not as runtime error
    /// and panic
    pub fn from_new_and_old(new: ParserOutput, old: ParserOutput) -> Self {
        let metrics_summary =
            ToolMetricSummary::try_from_new_and_old_metrics(&new.metrics, &old.metrics)
                .expect("New and old metrics should have a matching kind");
        Self {
            details: EitherOrBoth::Both(new.into(), old.into()),
            metrics_summary,
        }
    }
}

impl ProfileTotal {
    pub fn is_regressed(&self) -> bool {
        !self.regressions.is_empty()
    }

    pub fn is_some(&self) -> bool {
        self.summary.is_some()
    }

    pub fn is_none(&self) -> bool {
        self.summary.is_none()
    }
}

impl Profiles {
    pub fn new(values: Vec<Profile>) -> Self {
        Self(values)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Profile> {
        self.0.iter()
    }

    pub fn push(&mut self, summary: Profile) {
        self.0.push(summary);
    }

    pub fn is_regressed(&self) -> bool {
        self.iter().any(Profile::is_regressed)
    }
}

impl IntoIterator for Profiles {
    type Item = Profile;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl SummaryOutput {
    /// Create a new `SummaryOutput` with `dir` as base dir and an extension fitting the
    /// [`SummaryFormat`]
    pub fn new(format: SummaryFormat, dir: &Path) -> Self {
        Self {
            format,
            path: dir.join("summary.json"),
        }
    }

    /// Initialize this `SummaryOutput` removing old summary files
    pub fn init(&self) -> Result<()> {
        for entry in glob(self.path.with_extension("*").to_string_lossy().as_ref())
            .expect("Glob pattern should be valid")
        {
            let entry = entry?;
            std::fs::remove_file(entry.as_path()).with_context(|| {
                format!(
                    "Failed removing summary file '{}'",
                    entry.as_path().display()
                )
            })?;
        }

        Ok(())
    }

    /// Try to create an empty summary file returning the [`File`] object
    pub fn create(&self) -> Result<File> {
        File::create(&self.path).with_context(|| "Failed to create json summary file")
    }
}

impl ToolMetricSummary {
    pub fn add_mut(&mut self, other: &Self) {
        match (self, other) {
            (ToolMetricSummary::ErrorTool(this), ToolMetricSummary::ErrorTool(other)) => {
                this.add(other);
            }
            (ToolMetricSummary::Dhat(this), ToolMetricSummary::Dhat(other)) => {
                this.add(other);
            }
            (ToolMetricSummary::Callgrind(this), ToolMetricSummary::Callgrind(other)) => {
                this.add(other);
            }
            (ToolMetricSummary::Cachegrind(this), ToolMetricSummary::Cachegrind(other)) => {
                this.add(other);
            }
            _ => {}
        }
    }

    pub fn from_new_metrics(metrics: &ToolMetrics) -> Self {
        match metrics {
            ToolMetrics::None => ToolMetricSummary::None,
            ToolMetrics::Dhat(metrics) => {
                ToolMetricSummary::Dhat(MetricsSummary::new(EitherOrBoth::Left(metrics.clone())))
            }
            ToolMetrics::ErrorTool(metrics) => ToolMetricSummary::ErrorTool(MetricsSummary::new(
                EitherOrBoth::Left(metrics.clone()),
            )),
            ToolMetrics::Callgrind(metrics) => ToolMetricSummary::Callgrind(MetricsSummary::new(
                EitherOrBoth::Left(metrics.clone()),
            )),
            ToolMetrics::Cachegrind(metrics) => ToolMetricSummary::Cachegrind(MetricsSummary::new(
                EitherOrBoth::Left(metrics.clone()),
            )),
        }
    }
    pub fn from_old_metrics(metrics: &ToolMetrics) -> Self {
        match metrics {
            ToolMetrics::None => ToolMetricSummary::None,
            ToolMetrics::Dhat(metrics) => {
                ToolMetricSummary::Dhat(MetricsSummary::new(EitherOrBoth::Right(metrics.clone())))
            }
            ToolMetrics::ErrorTool(metrics) => ToolMetricSummary::ErrorTool(MetricsSummary::new(
                EitherOrBoth::Right(metrics.clone()),
            )),
            ToolMetrics::Callgrind(metrics) => ToolMetricSummary::Callgrind(MetricsSummary::new(
                EitherOrBoth::Right(metrics.clone()),
            )),
            ToolMetrics::Cachegrind(metrics) => ToolMetricSummary::Cachegrind(MetricsSummary::new(
                EitherOrBoth::Right(metrics.clone()),
            )),
        }
    }

    /// Return the `ToolMetricSummary` if the `MetricsKind` are the same kind, else return with
    /// error
    pub fn try_from_new_and_old_metrics(
        new_metrics: &ToolMetrics,
        old_metrics: &ToolMetrics,
    ) -> Result<Self> {
        match (new_metrics, old_metrics) {
            (ToolMetrics::None, ToolMetrics::None) => Ok(ToolMetricSummary::None),
            (ToolMetrics::Dhat(new_metrics), ToolMetrics::Dhat(old_metrics)) => {
                Ok(ToolMetricSummary::Dhat(MetricsSummary::new(
                    EitherOrBoth::Both(new_metrics.clone(), old_metrics.clone()),
                )))
            }
            (ToolMetrics::ErrorTool(new_metrics), ToolMetrics::ErrorTool(old_metrics)) => {
                Ok(ToolMetricSummary::ErrorTool(MetricsSummary::new(
                    EitherOrBoth::Both(new_metrics.clone(), old_metrics.clone()),
                )))
            }
            (ToolMetrics::Callgrind(new_metrics), ToolMetrics::Callgrind(old_metrics)) => {
                Ok(ToolMetricSummary::Callgrind(MetricsSummary::new(
                    EitherOrBoth::Both(new_metrics.clone(), old_metrics.clone()),
                )))
            }
            (ToolMetrics::Cachegrind(new_metrics), ToolMetrics::Cachegrind(old_metrics)) => {
                Ok(ToolMetricSummary::Cachegrind(MetricsSummary::new(
                    EitherOrBoth::Both(new_metrics.clone(), old_metrics.clone()),
                )))
            }
            _ => Err(anyhow!("Cannot create summary from incompatible costs")),
        }
    }

    pub fn from_self_and_other(this: &Self, other: &Self) -> Option<Self> {
        match (this, other) {
            (ToolMetricSummary::None, ToolMetricSummary::None) => Some(ToolMetricSummary::None),
            (
                ToolMetricSummary::Callgrind(metrics),
                ToolMetricSummary::Callgrind(other_metrics),
            ) => {
                let costs = metrics.extract_costs();
                let other_costs = other_metrics.extract_costs();

                if let (
                    EitherOrBoth::Left(new) | EitherOrBoth::Both(new, _),
                    EitherOrBoth::Left(other_new) | EitherOrBoth::Both(other_new, _),
                ) = (costs, other_costs)
                {
                    Some(ToolMetricSummary::Callgrind(MetricsSummary::new(
                        EitherOrBoth::Both(new, other_new),
                    )))
                } else {
                    None
                }
            }
            (
                ToolMetricSummary::ErrorTool(metrics),
                ToolMetricSummary::ErrorTool(other_metrics),
            ) => {
                let costs = metrics.extract_costs();
                let other_costs = other_metrics.extract_costs();

                if let (
                    EitherOrBoth::Left(new) | EitherOrBoth::Both(new, _),
                    EitherOrBoth::Left(other_new) | EitherOrBoth::Both(other_new, _),
                ) = (costs, other_costs)
                {
                    Some(ToolMetricSummary::ErrorTool(MetricsSummary::new(
                        EitherOrBoth::Both(new, other_new),
                    )))
                } else {
                    None
                }
            }
            (ToolMetricSummary::Dhat(metrics), ToolMetricSummary::Dhat(other_metrics)) => {
                let costs = metrics.extract_costs();
                let other_costs = other_metrics.extract_costs();

                if let (
                    EitherOrBoth::Left(new) | EitherOrBoth::Both(new, _),
                    EitherOrBoth::Left(other_new) | EitherOrBoth::Both(other_new, _),
                ) = (costs, other_costs)
                {
                    Some(ToolMetricSummary::Dhat(MetricsSummary::new(
                        EitherOrBoth::Both(new, other_new),
                    )))
                } else {
                    None
                }
            }
            (
                ToolMetricSummary::Cachegrind(metrics),
                ToolMetricSummary::Cachegrind(other_metrics),
            ) => {
                let costs = metrics.extract_costs();
                let other_costs = other_metrics.extract_costs();

                if let (
                    EitherOrBoth::Left(new) | EitherOrBoth::Both(new, _),
                    EitherOrBoth::Left(other_new) | EitherOrBoth::Both(other_new, _),
                ) = (costs, other_costs)
                {
                    Some(ToolMetricSummary::Cachegrind(MetricsSummary::new(
                        EitherOrBoth::Both(new, other_new),
                    )))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use EventKind::*;

    use super::*;

    fn expected_metrics_diff<D>(metrics: EitherOrBoth<u64>, diffs: D) -> MetricsDiff
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

    fn metrics_summary_fixture<T>(kinds: &[(EitherOrBoth<u64>, T)]) -> MetricsSummary<EventKind>
    where
        T: Into<Option<(f64, f64)>> + Clone,
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
            .zip(kinds.iter())
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
        let expected = expected_metrics_diff(metrics.clone(), expected_diffs);
        let actual = MetricsDiff::new(metrics);

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
        let new_diff = MetricsDiff::new(metric);
        let old_diff = MetricsDiff::new(other_metric);
        let expected = MetricsDiff::new(expected);

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
        let expected_metrics_summary = metrics_summary_fixture(expected);
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
