use std::borrow::Cow;
use std::ffi::OsString;
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
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::callgrind::Summaries;
use super::common::ModulePath;
use super::format::{Formatter, OutputFormat, OutputFormatKind, VerticalFormat};
use super::metrics::Metrics;
use super::tool::ValgrindTool;
use crate::api::{DhatMetricKind, ErrorMetricKind, EventKind};
use crate::error::Error;
use crate::runner::metrics::Summarize;
use crate::util::{factor_diff, make_absolute, percentage_diff, EitherOrBoth};

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

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct BaselineName(String);

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
#[derive(Debug, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, PartialEq, Serialize, Deserialize)]
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
    /// The summary of the callgrind run
    pub callgrind_summary: Option<CallgrindSummary>,
    /// The summary of other valgrind tool runs
    pub tool_summaries: Vec<ToolSummary>,
}

/// The `CallgrindRegression` describing a single event based performance regression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CallgrindRegression {
    /// The [`EventKind`] which is affected by a performance regression
    pub event_kind: EventKind,
    /// The value of the new benchmark run
    pub new: u64,
    /// The value of the old benchmark run
    pub old: u64,
    /// The difference between new and old in percent
    pub diff_pct: f64,
    /// The value of the limit which was exceeded to cause a performance regression
    pub limit: f64,
}

/// The `CallgrindRun` contains all `CallgrindRunSegments` and their total costs in a
/// `CallgrindTotal`.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CallgrindRun {
    /// All `CallgrindRunSummary`s
    pub segments: Vec<CallgrindRunSegment>,
    /// The total costs of all `CallgrindRunSummary`s in this `CallgrindRunSummaries`
    pub total: CallgrindTotal,
}

/// The `CallgrindRunSegment` containing the metric differences, performance regressions of a
/// callgrind run segment.
///
/// A segment can be a part (caused by options like `--dump-every-bb=xxx`), a thread (caused by
/// `--separate-threads`) or a pid (possibly caused by `--trace-children`). A segment is a summary
/// over a single file which contains the costs of that part, thread and/or pid.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CallgrindRunSegment {
    /// The executed command extracted from Valgrind output
    pub command: String,
    /// If present, the `Baseline` used to compare the new with the old output
    pub baseline: Option<Baseline>,
    /// All recorded metrics for the `EventKinds`
    pub events: MetricsSummary<EventKind>,
    /// All detected performance regressions per callgrind run
    pub regressions: Vec<CallgrindRegression>,
}

/// The total callgrind costs over the `CallgrindRunSegments` and all detected regressions for the
/// total
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CallgrindTotal {
    /// The total over the segment metrics
    pub summary: MetricsSummary,
    /// All detected regressions for the total metrics
    pub regressions: Vec<CallgrindRegression>,
}

/// The `CallgrindSummary` contains the callgrind run, flamegraph paths and other paths to the
/// segments of the callgrind run.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CallgrindSummary {
    /// The paths to the `*.log` files
    pub log_paths: Vec<PathBuf>,
    /// The paths to the `*.out` files
    pub out_paths: Vec<PathBuf>,
    /// The summaries of possibly created flamegraphs
    pub flamegraphs: Vec<FlamegraphSummary>,
    /// The summary of all callgrind segments is a `CallgrindRun`
    pub callgrind_run: CallgrindRun,
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
    DhatMetrics(Metrics<DhatMetricKind>),
    /// The metrics of a tool run which reports errors (memcheck, helgrind, drd)
    ErrorMetrics(Metrics<ErrorMetricKind>),
    /// The metrics of a callgrind benchmark
    CallgrindMetrics(Metrics<EventKind>),
}

/// The `MetricsSummary` contains all differences between two tool run segments
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct MetricsSummary<K: Hash + Eq = EventKind>(IndexMap<K, MetricsDiff>);

/// The `ToolMetricSummary` contains the `MetricsSummary` distinguished by tool and metric kinds
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum ToolMetricSummary {
    /// If there are no costs extracted (currently massif, bbv)
    #[default]
    None,
    /// The error summary of tools which reports errors (memcheck, helgrind, drd)
    ErrorSummary(MetricsSummary<ErrorMetricKind>),
    /// The dhat summary
    DhatSummary(MetricsSummary<DhatMetricKind>),
    /// The callgrind summary
    CallgrindSummary(MetricsSummary<EventKind>),
}

/// The differences between two `Costs` as percentage and factor
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Diffs {
    /// The percentage of the difference between two `Metrics`
    pub diff_pct: f64,
    /// The factor of the difference between two `Metrics`
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
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct SummaryOutput {
    /// The [`SummaryFormat`]
    format: SummaryFormat,
    /// The path to the destination file of this summary
    path: PathBuf,
}

/// Some additional and necessary information about the tool run segment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AsRef)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct SegmentDetails {
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

/// The `ToolRun` contains all information about a single tool run with possibly multiple segments
///
/// The total is always present and summarizes all tool run segments. In the special case of a
/// single tool run segment, the total equals the metrics of this segment.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ToolRun {
    /// All `ToolRunSegment`s
    pub segments: Vec<ToolRunSegment>,
    /// The total over the `ToolRunSegment`s
    pub total: ToolMetricSummary,
}

/// A single segment of a tool run and if present the comparison with the "old" segment
///
/// A tool run can produce multiple segments, for example for each process and subprocess with
/// (--trace-children).
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ToolRunSegment {
    /// The details (like command, thread number etc.) about the segment(s)
    pub details: EitherOrBoth<SegmentDetails>,
    /// The `ToolMetricSummary`
    pub metrics_summary: ToolMetricSummary,
}

/// The `ToolSummary` containing all information about a valgrind tool run
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ToolSummary {
    /// The Valgrind tool like `DHAT`, `Memcheck` etc.
    pub tool: ValgrindTool,
    /// The paths to the `*.log` files. All tools produce at least one log file
    pub log_paths: Vec<PathBuf>,
    /// The paths to the `*.out` files. Not all tools produce an output in addition to the log
    /// files
    pub out_paths: Vec<PathBuf>,
    /// The metrics and details about the tool run
    pub summaries: ToolRun,
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

impl Display for BaselineName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
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
            callgrind_summary: None,
            tool_summaries: vec![],
            summary_output: output,
            project_root,
            package_dir,
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
    /// If the regressions are configured to be not `fail_fast` and there is a regressions, then the
    /// `is_regressed` variable is updated to true.
    ///
    /// # Errors
    ///
    /// If the regressions are configured to be `fail_fast` an error is returned
    pub fn check_regression(&self, is_regressed: &mut bool, fail_fast: bool) -> Result<()> {
        if let Some(callgrind_summary) = &self.callgrind_summary {
            let benchmark_is_regressed = callgrind_summary.is_regressed();
            if benchmark_is_regressed && fail_fast {
                return Err(Error::RegressionError(true).into());
            }

            *is_regressed |= benchmark_is_regressed;
        }

        Ok(())
    }

    // TODO: Compare not only the total??
    // TODO: Compare dhat
    pub fn compare_and_print(
        &self,
        id: &str,
        other: &Self,
        output_format: &OutputFormat,
    ) -> Result<()> {
        if let (Some(callgrind_summary), Some(other_callgrind_summary)) =
            (&self.callgrind_summary, &other.callgrind_summary)
        {
            if let (
                EitherOrBoth::Left(new) | EitherOrBoth::Both(new, _),
                EitherOrBoth::Left(other_new) | EitherOrBoth::Both(other_new, _),
            ) = (
                callgrind_summary
                    .callgrind_run
                    .total
                    .summary
                    .extract_costs(),
                other_callgrind_summary
                    .callgrind_run
                    .total
                    .summary
                    .extract_costs(),
            ) {
                let new_summary = MetricsSummary::new(EitherOrBoth::Both(new, other_new));
                VerticalFormat.print_comparison(
                    &self.function_name,
                    id,
                    self.details.as_deref(),
                    &ToolMetricSummary::CallgrindSummary(new_summary),
                    output_format,
                )?;
            }
        }

        Ok(())
    }
}

impl CallgrindSummary {
    /// Create a new `CallgrindSummary`
    pub fn new(log_paths: Vec<PathBuf>, out_paths: Vec<PathBuf>) -> CallgrindSummary {
        Self {
            log_paths,
            out_paths,
            flamegraphs: Vec::default(),
            callgrind_run: CallgrindRun::default(),
        }
    }

    /// Return true if there are any recorded regressions in this `CallgrindSummary`
    pub fn is_regressed(&self) -> bool {
        self.callgrind_run
            .segments
            .iter()
            .any(|r| !r.regressions.is_empty())
    }

    pub fn add_summaries(
        &mut self,
        bench_bin: &Path,
        bench_args: &[OsString],
        baselines: &(Option<String>, Option<String>),
        summaries: Summaries,
        regressions: Vec<CallgrindRegression>,
    ) {
        let command = format!(
            "{} {}",
            bench_bin.display(),
            shlex::try_join(
                bench_args
                    .iter()
                    .map(|s| s.to_string_lossy().to_string())
                    .collect::<Vec<String>>()
                    .as_slice()
                    .iter()
                    .map(String::as_str)
            )
            .unwrap()
        );
        for summary in summaries.summaries {
            let old_baseline = match summary.details {
                EitherOrBoth::Left(_) => None,
                EitherOrBoth::Both(_, old) | EitherOrBoth::Right(old) => Some(Baseline {
                    kind: baselines.1.as_ref().map_or(BaselineKind::Old, |name| {
                        BaselineKind::Name(BaselineName(name.to_owned()))
                    }),
                    path: old.0,
                }),
            };

            self.callgrind_run.segments.push(CallgrindRunSegment {
                command: command.clone(),
                baseline: old_baseline,
                events: summary.metrics_summary,
                regressions: vec![],
            });
        }

        self.callgrind_run.total.summary = summaries.total.clone();
        self.callgrind_run.total.regressions = regressions;
    }
}

impl MetricsDiff {
    pub fn new(costs: EitherOrBoth<u64>) -> Self {
        if let EitherOrBoth::Both(new, old) = costs {
            Self {
                metrics: costs,
                diffs: Some(Diffs::new(new, old)),
            }
        } else {
            Self {
                metrics: costs,
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

impl ToolMetricSummary {
    pub fn add_mut(&mut self, other: &Self) {
        match (self, other) {
            (ToolMetricSummary::ErrorSummary(this), ToolMetricSummary::ErrorSummary(other)) => {
                this.add(other);
            }
            (ToolMetricSummary::DhatSummary(this), ToolMetricSummary::DhatSummary(other)) => {
                this.add(other);
            }
            (
                ToolMetricSummary::CallgrindSummary(this),
                ToolMetricSummary::CallgrindSummary(other),
            ) => {
                this.add(other);
            }
            _ => {}
        }
    }

    pub fn from_new_costs(costs: &ToolMetrics) -> Self {
        match costs {
            ToolMetrics::None => ToolMetricSummary::None,
            ToolMetrics::DhatMetrics(costs) => ToolMetricSummary::DhatSummary(MetricsSummary::new(
                EitherOrBoth::Left(costs.clone()),
            )),
            ToolMetrics::ErrorMetrics(costs) => ToolMetricSummary::ErrorSummary(
                MetricsSummary::new(EitherOrBoth::Left(costs.clone())),
            ),
            ToolMetrics::CallgrindMetrics(costs) => ToolMetricSummary::CallgrindSummary(
                MetricsSummary::new(EitherOrBoth::Left(costs.clone())),
            ),
        }
    }
    pub fn from_old_costs(costs: &ToolMetrics) -> Self {
        match costs {
            ToolMetrics::None => ToolMetricSummary::None,
            ToolMetrics::DhatMetrics(costs) => ToolMetricSummary::DhatSummary(MetricsSummary::new(
                EitherOrBoth::Right(costs.clone()),
            )),
            ToolMetrics::ErrorMetrics(costs) => ToolMetricSummary::ErrorSummary(
                MetricsSummary::new(EitherOrBoth::Right(costs.clone())),
            ),
            ToolMetrics::CallgrindMetrics(costs) => ToolMetricSummary::CallgrindSummary(
                MetricsSummary::new(EitherOrBoth::Right(costs.clone())),
            ),
        }
    }

    /// Return the `CostsSummaryType` if the `CostsKind` have the same kind, else return with error
    pub fn try_from_new_and_old_costs(
        new_costs: &ToolMetrics,
        old_costs: &ToolMetrics,
    ) -> Result<Self> {
        match (new_costs, old_costs) {
            (ToolMetrics::None, ToolMetrics::None) => Ok(ToolMetricSummary::None),
            (ToolMetrics::DhatMetrics(new_costs), ToolMetrics::DhatMetrics(old_costs)) => {
                Ok(ToolMetricSummary::DhatSummary(MetricsSummary::new(
                    EitherOrBoth::Both(new_costs.clone(), old_costs.clone()),
                )))
            }
            (ToolMetrics::ErrorMetrics(new_costs), ToolMetrics::ErrorMetrics(old_costs)) => {
                Ok(ToolMetricSummary::ErrorSummary(MetricsSummary::new(
                    EitherOrBoth::Both(new_costs.clone(), old_costs.clone()),
                )))
            }
            (
                ToolMetrics::CallgrindMetrics(new_costs),
                ToolMetrics::CallgrindMetrics(old_costs),
            ) => Ok(ToolMetricSummary::CallgrindSummary(MetricsSummary::new(
                EitherOrBoth::Both(new_costs.clone(), old_costs.clone()),
            ))),
            _ => Err(anyhow!("Cannot create summary from incompatible costs")),
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

impl<K> MetricsSummary<K>
where
    K: Hash + Eq + Summarize + Display + Clone,
{
    /// TODO: TEST
    /// Create a new `CostsSummary` calculating the differences between new and old (if any)
    /// [`Costs`]
    pub fn new(costs: EitherOrBoth<Metrics<K>>) -> Self {
        match costs {
            EitherOrBoth::Left(new) => {
                let mut new = Cow::Owned(new);
                K::summarize(&mut new);

                Self(
                    new.iter()
                        .map(|(event_kind, cost)| {
                            (
                                event_kind.clone(),
                                MetricsDiff::new(EitherOrBoth::Left(*cost)),
                            )
                        })
                        .collect::<IndexMap<_, _>>(),
                )
            }
            EitherOrBoth::Right(old) => {
                let mut old = Cow::Owned(old);
                K::summarize(&mut old);

                Self(
                    old.iter()
                        .map(|(event_kind, cost)| {
                            (
                                event_kind.clone(),
                                MetricsDiff::new(EitherOrBoth::Right(*cost)),
                            )
                        })
                        .collect::<IndexMap<_, _>>(),
                )
            }
            EitherOrBoth::Both(new, old) => {
                let mut new = Cow::Owned(new);
                K::summarize(&mut new);
                let mut old = Cow::Owned(old);
                K::summarize(&mut old);

                let mut map = indexmap! {};
                for event_kind in new.metric_kinds_union(&old) {
                    let diff = match (
                        new.metric_by_kind(&event_kind),
                        old.metric_by_kind(&event_kind),
                    ) {
                        (Some(cost), None) => MetricsDiff::new(EitherOrBoth::Left(cost)),
                        (None, Some(cost)) => MetricsDiff::new(EitherOrBoth::Right(cost)),
                        (Some(new), Some(old)) => MetricsDiff::new(EitherOrBoth::Both(new, old)),
                        (None, None) => {
                            unreachable!(
                                "The union contains the event kinds either from new or old or \
                                 from both"
                            )
                        }
                    };
                    map.insert(event_kind, diff);
                }
                Self(map)
            }
        }
    }

    /// Try to return a [`CostsDiff`] for the specified `EventKind`
    pub fn diff_by_kind(&self, event_kind: &K) -> Option<&MetricsDiff> {
        self.0.get(event_kind)
    }

    pub fn all_diffs(&self) -> impl Iterator<Item = (&K, &MetricsDiff)> {
        self.0.iter()
    }

    pub fn extract_costs(&self) -> EitherOrBoth<Metrics<K>> {
        let mut new_costs: Metrics<K> = Metrics::empty();
        let mut old_costs: Metrics<K> = Metrics::empty();
        // The diffs should not be empty
        for (event_kind, diff) in self.all_diffs() {
            match diff.metrics {
                EitherOrBoth::Left(new) => {
                    new_costs.insert(event_kind.clone(), new);
                }
                EitherOrBoth::Right(old) => {
                    old_costs.insert(event_kind.clone(), old);
                }
                EitherOrBoth::Both(new, old) => {
                    new_costs.insert(event_kind.clone(), new);
                    old_costs.insert(event_kind.clone(), old);
                }
            }
        }

        match (new_costs.is_empty(), old_costs.is_empty()) {
            (false, false) => EitherOrBoth::Both(new_costs, old_costs),
            (false, true) => EitherOrBoth::Left(new_costs),
            (true, false) => EitherOrBoth::Right(old_costs),
            (true, true) => unreachable!("A costs diff contains new or old values or both."),
        }
    }

    // TODO: TEST
    // TODO: RENAME TO add_mut
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

impl ToolMetricSummary {
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
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

impl ToolRun {
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn has_multiple(&self) -> bool {
        self.segments.len() > 1
    }
}

impl ToolRunSegment {
    pub fn new_has_errors(&self) -> bool {
        match &self.metrics_summary {
            ToolMetricSummary::None
            | ToolMetricSummary::DhatSummary(_)
            | ToolMetricSummary::CallgrindSummary(_) => false,
            ToolMetricSummary::ErrorSummary(costs) => costs
                .diff_by_kind(&ErrorMetricKind::Errors)
                .map_or(false, |e| match e.metrics {
                    EitherOrBoth::Left(new) | EitherOrBoth::Both(new, _) => new > 0,
                    EitherOrBoth::Right(_) => false,
                }),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

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
    fn test_costs_diff_add(
        #[case] cost: EitherOrBoth<u64>,
        #[case] other_cost: EitherOrBoth<u64>,
        #[case] expected: EitherOrBoth<u64>,
    ) {
        let new_diff = MetricsDiff::new(cost);
        let old_diff = MetricsDiff::new(other_cost);
        let expected = MetricsDiff::new(expected);

        assert_eq!(new_diff.add(&old_diff), expected);
        assert_eq!(old_diff.add(&new_diff), expected);
    }
}
