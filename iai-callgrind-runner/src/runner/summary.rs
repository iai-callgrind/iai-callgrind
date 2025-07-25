//! The summary of a benchmark run

use std::fmt::{Debug, Display};
use std::fs::File;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use derive_more::AsRef;
use glob::glob;
use itertools::Itertools;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::{Baselines, ModulePath};
use super::format::{Formatter, OutputFormat, OutputFormatKind, VerticalFormatter};
use super::metrics::{Metric, MetricKind, Metrics, MetricsSummary};
use super::tool::parser::ParserOutput;
use super::tool::regression::RegressionMetrics;
use crate::api::{CachegrindMetric, DhatMetric, ErrorMetric, EventKind, ValgrindTool};
use crate::error::Error;
use crate::util::{factor_diff, make_absolute, percentage_diff, EitherOrBoth};

/// The version of the summary json schema
pub const SCHEMA_VERSION: &str = "6";

/// The `BaselineKind` describing the baseline
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum BaselineKind {
    /// Compare new against `*.old` output files
    Old,
    /// Compare new against a named baseline
    Name(BaselineName),
}

/// The `BenchmarkKind`, differentiating between library and binary benchmarks
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum BenchmarkKind {
    /// A library benchmark
    LibraryBenchmark,
    /// A binary benchmark
    BinaryBenchmark,
}

/// The format (json, ...) in which the summary file should be saved or printed
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum SummaryFormat {
    /// The format in a space optimal json representation without newlines
    Json,
    /// The format in pretty printed json
    PrettyJson,
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

/// The metrics distinguished per tool class
///
/// The tool classes are: dhat, error metrics from memcheck, drd, helgrind and callgrind
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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

/// A detected performance regression depending on the limit either `Soft` or `Hard`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum ToolRegression {
    /// A performance regression triggered by a soft limit
    Soft {
        /// The metric kind per tool
        metric: MetricKind,
        /// The value of the new benchmark run
        new: Metric,
        /// The value of the old benchmark run
        old: Metric,
        /// The difference between new and old in percent. Serialized as string to preserve
        /// infinity values and avoid null in json.
        #[serde(with = "crate::serde::float_64")]
        #[cfg_attr(feature = "schema", schemars(with = "String"))]
        diff_pct: f64,
        /// The value of the limit which was exceeded to cause a performance regression. Serialized
        /// as string to preserve infinity values and avoid null in json.
        #[serde(with = "crate::serde::float_64")]
        #[cfg_attr(feature = "schema", schemars(with = "String"))]
        limit: f64,
    },
    /// A performance regression triggered by a hard limit
    Hard {
        /// The metric kind per tool
        metric: MetricKind,
        /// The value of the benchmark run
        new: Metric,
        /// The difference between new and the limit
        diff: Metric,
        /// The limit
        limit: Metric,
    },
}

/// A `Baseline` depending on the [`BaselineKind`] which points to the corresponding path
///
/// This baseline is used for comparisons with the new output of valgrind tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Baseline {
    /// The kind of the `Baseline`
    pub kind: BaselineKind,
    /// The path to the file which is used to compare against the new output
    pub path: PathBuf,
}

/// The name of the baseline
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct BaselineName(String);

/// The `BenchmarkSummary` containing all the information of a single benchmark run
///
/// This includes produced files, recorded callgrind events, performance regressions ...
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct BenchmarkSummary {
    /// The baselines if any. An absent first baseline indicates that new output was produced. An
    /// absent second baseline indicates the usage of the usual "*.old" output.
    pub baselines: (Option<String>, Option<String>),
    /// The path to the binary which is executed by valgrind. In case of a library benchmark this
    /// is the compiled benchmark file. In case of a binary benchmark this is the path to the
    /// command.
    pub benchmark_exe: PathBuf,
    /// The path to the benchmark file
    pub benchmark_file: PathBuf,
    /// More details describing this benchmark run
    pub details: Option<String>,
    /// The name of the function under test
    pub function_name: String,
    /// The user provided id of this benchmark
    pub id: Option<String>,
    /// Whether this summary describes a library or binary benchmark
    pub kind: BenchmarkKind,
    /// The rust path in the form `bench_file::group::bench`
    pub module_path: String,
    /// The directory of the package
    pub package_dir: PathBuf,
    /// The summary of other valgrind tool runs
    pub profiles: Profiles,
    /// The project's root directory
    pub project_root: PathBuf,
    /// The destination and kind of the summary file
    pub summary_output: Option<SummaryOutput>,
    /// The version of this format. Only backwards incompatible changes cause an increase of the
    /// version
    pub version: String,
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
    /// If present, the path to the file of the old regular (non-differential) flamegraph
    pub base_path: Option<PathBuf>,
    /// If present, the path to the file of the differential flamegraph
    pub diff_path: Option<PathBuf>,
    /// The `EventKind` of the flamegraph
    pub event_kind: EventKind,
    /// If present, the path to the file of the regular (non-differential) flamegraph
    pub regular_path: Option<PathBuf>,
}

/// The `ToolSummary` containing all information about a valgrind tool run
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Profile {
    /// Details and information about the created flamegraphs if any
    pub flamegraphs: Vec<FlamegraphSummary>,
    /// The paths to the `*.log` files. All tools produce at least one log file
    pub log_paths: Vec<PathBuf>,
    /// The paths to the `*.out` files. Not all tools produce an output in addition to the log
    /// files
    pub out_paths: Vec<PathBuf>,
    /// The metrics and details about the tool run
    pub summaries: ProfileData,
    /// The Valgrind tool like `DHAT`, `Memcheck` etc.
    pub tool: ValgrindTool,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRef)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ProfileInfo {
    /// The executed command extracted from Valgrind output
    pub command: String,
    /// More details for example from the logging output of the tool run
    pub details: Option<String>,
    /// The parent pid of this process
    pub parent_pid: Option<i32>,
    /// The part of this tool run (only callgrind)
    pub part: Option<u64>,
    /// The path to the file from the tool run
    pub path: PathBuf,
    /// The pid of this process
    pub pid: i32,
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
    /// The detected regressions if any
    pub regressions: Vec<ToolRegression>,
    /// The summary of metrics of the tool
    pub summary: ToolMetricSummary,
}

/// The collection of all generated [`Profile`]s
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
#[derive(Default)]
pub struct Profiles(Vec<Profile>);

/// Manage the summary output file with this `SummaryOutput`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct SummaryOutput {
    /// The [`SummaryFormat`]
    format: SummaryFormat,
    /// The path to the destination file of this summary
    path: PathBuf,
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
            version: SCHEMA_VERSION.to_owned(),
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

    /// If the summary is json output, print it and eventually safe it, if configured to do so
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

    /// Compare this summary with another and print the result of the comparison
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
    /// Create a new `Diffs` calculating the percentage and factor from the `new` and `old` metrics
    pub fn new(new: Metric, old: Metric) -> Self {
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

impl Profile {
    /// Return true if one of the summaries has regressed
    pub fn is_regressed(&self) -> bool {
        self.summaries.is_regressed()
    }
}

impl ProfileData {
    /// Return true if the profile data is empty
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// Return true if the total and only the total has regressed
    pub fn is_regressed(&self) -> bool {
        self.total.is_regressed()
    }

    /// Return true if there are multiple parts
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
    /// Return true if an error checking valgrind tool (like `Memcheck`) has errors detected
    pub fn new_has_errors(&self) -> bool {
        match &self.metrics_summary {
            ToolMetricSummary::None
            | ToolMetricSummary::Dhat(_)
            | ToolMetricSummary::Cachegrind(_)
            | ToolMetricSummary::Callgrind(_) => false,
            ToolMetricSummary::ErrorTool(metrics) => metrics
                .diff_by_kind(&ErrorMetric::Errors)
                .is_some_and(|e| match e.metrics {
                    EitherOrBoth::Left(new) | EitherOrBoth::Both(new, _) => new > Metric::Int(0),
                    EitherOrBoth::Right(_) => false,
                }),
        }
    }

    /// Create a new part from `new` parser output
    pub fn from_new(new: ParserOutput) -> Self {
        let metrics_summary = ToolMetricSummary::from_new_metrics(&new.metrics);
        Self {
            details: EitherOrBoth::Left(new.into()),
            metrics_summary,
        }
    }

    /// Create a new part from `old` parser output
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
    /// Return true if there are any regressions
    pub fn is_regressed(&self) -> bool {
        !self.regressions.is_empty()
    }

    /// Return true if there is a summary
    pub fn is_some(&self) -> bool {
        self.summary.is_some()
    }

    /// Return true if there is no summary
    pub fn is_none(&self) -> bool {
        self.summary.is_none()
    }
}

impl Profiles {
    /// Create a new collection of [`Profile`]s
    pub fn new(values: Vec<Profile>) -> Self {
        Self(values)
    }

    /// Return an iterator over the contained [`Profile`]s
    pub fn iter(&self) -> impl Iterator<Item = &Profile> {
        self.0.iter()
    }

    /// Add a new [`Profile`] to this collection
    pub fn push(&mut self, summary: Profile) {
        self.0.push(summary);
    }

    /// Return true if any [`Profile`] has regressed
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
    /// Sum up another summary metrics to these metrics
    pub fn add_mut(&mut self, other: &Self) {
        match (self, other) {
            (Self::ErrorTool(this), Self::ErrorTool(other)) => {
                this.add(other);
            }
            (Self::Dhat(this), Self::Dhat(other)) => {
                this.add(other);
            }
            (Self::Callgrind(this), Self::Callgrind(other)) => {
                this.add(other);
            }
            (Self::Cachegrind(this), Self::Cachegrind(other)) => {
                this.add(other);
            }
            _ => {}
        }
    }

    /// Create a new summary from `new` [`ToolMetrics`]
    pub fn from_new_metrics(metrics: &ToolMetrics) -> Self {
        match metrics {
            ToolMetrics::None => Self::None,
            ToolMetrics::Dhat(metrics) => {
                Self::Dhat(MetricsSummary::new(EitherOrBoth::Left(metrics.clone())))
            }
            ToolMetrics::ErrorTool(metrics) => {
                Self::ErrorTool(MetricsSummary::new(EitherOrBoth::Left(metrics.clone())))
            }
            ToolMetrics::Callgrind(metrics) => {
                Self::Callgrind(MetricsSummary::new(EitherOrBoth::Left(metrics.clone())))
            }
            ToolMetrics::Cachegrind(metrics) => {
                Self::Cachegrind(MetricsSummary::new(EitherOrBoth::Left(metrics.clone())))
            }
        }
    }

    /// Create a new summary from `old` [`ToolMetrics`]
    pub fn from_old_metrics(metrics: &ToolMetrics) -> Self {
        match metrics {
            ToolMetrics::None => Self::None,
            ToolMetrics::Dhat(metrics) => {
                Self::Dhat(MetricsSummary::new(EitherOrBoth::Right(metrics.clone())))
            }
            ToolMetrics::ErrorTool(metrics) => {
                Self::ErrorTool(MetricsSummary::new(EitherOrBoth::Right(metrics.clone())))
            }
            ToolMetrics::Callgrind(metrics) => {
                Self::Callgrind(MetricsSummary::new(EitherOrBoth::Right(metrics.clone())))
            }
            ToolMetrics::Cachegrind(metrics) => {
                Self::Cachegrind(MetricsSummary::new(EitherOrBoth::Right(metrics.clone())))
            }
        }
    }

    /// Create a new summary from `new` and `old` [`ToolMetrics`]
    ///
    /// Return the `ToolMetricSummary` if the `MetricsKind` are the same kind, else return with
    /// error
    pub fn try_from_new_and_old_metrics(
        new_metrics: &ToolMetrics,
        old_metrics: &ToolMetrics,
    ) -> Result<Self> {
        match (new_metrics, old_metrics) {
            (ToolMetrics::None, ToolMetrics::None) => Ok(Self::None),
            (ToolMetrics::Dhat(new_metrics), ToolMetrics::Dhat(old_metrics)) => Ok(Self::Dhat(
                MetricsSummary::new(EitherOrBoth::Both(new_metrics.clone(), old_metrics.clone())),
            )),
            (ToolMetrics::ErrorTool(new_metrics), ToolMetrics::ErrorTool(old_metrics)) => {
                Ok(Self::ErrorTool(MetricsSummary::new(EitherOrBoth::Both(
                    new_metrics.clone(),
                    old_metrics.clone(),
                ))))
            }
            (ToolMetrics::Callgrind(new_metrics), ToolMetrics::Callgrind(old_metrics)) => {
                Ok(Self::Callgrind(MetricsSummary::new(EitherOrBoth::Both(
                    new_metrics.clone(),
                    old_metrics.clone(),
                ))))
            }
            (ToolMetrics::Cachegrind(new_metrics), ToolMetrics::Cachegrind(old_metrics)) => {
                Ok(Self::Cachegrind(MetricsSummary::new(EitherOrBoth::Both(
                    new_metrics.clone(),
                    old_metrics.clone(),
                ))))
            }
            _ => Err(anyhow!("Cannot create summary from incompatible costs")),
        }
    }

    /// Create a new summary from this summary and another [`ToolMetricSummary`]
    pub fn from_self_and_other(this: &Self, other: &Self) -> Option<Self> {
        match (this, other) {
            (Self::None, Self::None) => Some(Self::None),
            (Self::Callgrind(metrics), Self::Callgrind(other_metrics)) => {
                let costs = metrics.extract_costs();
                let other_costs = other_metrics.extract_costs();

                if let (
                    EitherOrBoth::Left(new) | EitherOrBoth::Both(new, _),
                    EitherOrBoth::Left(other_new) | EitherOrBoth::Both(other_new, _),
                ) = (costs, other_costs)
                {
                    Some(Self::Callgrind(MetricsSummary::new(EitherOrBoth::Both(
                        new, other_new,
                    ))))
                } else {
                    None
                }
            }
            (Self::ErrorTool(metrics), Self::ErrorTool(other_metrics)) => {
                let costs = metrics.extract_costs();
                let other_costs = other_metrics.extract_costs();

                if let (
                    EitherOrBoth::Left(new) | EitherOrBoth::Both(new, _),
                    EitherOrBoth::Left(other_new) | EitherOrBoth::Both(other_new, _),
                ) = (costs, other_costs)
                {
                    Some(Self::ErrorTool(MetricsSummary::new(EitherOrBoth::Both(
                        new, other_new,
                    ))))
                } else {
                    None
                }
            }
            (Self::Dhat(metrics), Self::Dhat(other_metrics)) => {
                let costs = metrics.extract_costs();
                let other_costs = other_metrics.extract_costs();

                if let (
                    EitherOrBoth::Left(new) | EitherOrBoth::Both(new, _),
                    EitherOrBoth::Left(other_new) | EitherOrBoth::Both(other_new, _),
                ) = (costs, other_costs)
                {
                    Some(Self::Dhat(MetricsSummary::new(EitherOrBoth::Both(
                        new, other_new,
                    ))))
                } else {
                    None
                }
            }
            (Self::Cachegrind(metrics), Self::Cachegrind(other_metrics)) => {
                let costs = metrics.extract_costs();
                let other_costs = other_metrics.extract_costs();

                if let (
                    EitherOrBoth::Left(new) | EitherOrBoth::Both(new, _),
                    EitherOrBoth::Left(other_new) | EitherOrBoth::Both(other_new, _),
                ) = (costs, other_costs)
                {
                    Some(Self::Cachegrind(MetricsSummary::new(EitherOrBoth::Both(
                        new, other_new,
                    ))))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Return true if this summary has metrics
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Return true if this summary doesn't have metrics (currently massif, bbv)
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl ToolRegression {
    /// Create a new `ToolRegression`
    pub fn with<T>(apply: fn(T) -> MetricKind, regressions: RegressionMetrics<T>) -> Self {
        match regressions {
            RegressionMetrics::Soft(metric, new, old, diff_pct, limit) => Self::Soft {
                metric: apply(metric),
                new,
                old,
                diff_pct,
                limit,
            },
            RegressionMetrics::Hard(metric, new, diff, limit) => Self::Hard {
                metric: apply(metric),
                new,
                diff,
                limit,
            },
        }
    }
}
