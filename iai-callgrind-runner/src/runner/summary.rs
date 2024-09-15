use std::borrow::Cow;
use std::ffi::OsString;
use std::fmt::{Debug, Display};
use std::fs::File;
use std::hash::Hash;
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result};
use glob::glob;
use indexmap::{indexmap, IndexMap, IndexSet};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::callgrind::Summaries;
use super::common::ModulePath;
use super::costs::Costs;
use super::format::{Formatter, OutputFormat, VerticalFormat};
use super::meta::Metadata;
use super::tool::{ToolOutputPath, ValgrindTool};
use crate::api::{DhatMetricKind, ErrorMetricKind, EventKind};
use crate::error::Error;
use crate::runner::costs::Summarize;
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

/// TODO: Add a None or Default variant ??
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

/// The `CallgrindRegressionSummary` describing a single event based performance regression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CallgrindRegressionSummary {
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

/// TODO: DOCS
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CallgrindRunSummaries {
    pub summaries: Vec<CallgrindRunSummary>,
    pub total: CallgrindTotal,
}

/// The `CallgrindRunSummary` containing the recorded events, performance regressions of a single
/// callgrind run
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CallgrindRunSummary {
    /// The executed command extracted from Valgrind output
    pub command: String,
    /// If present, the `Baseline` used to compare the new with the old output
    pub baseline: Option<Baseline>,
    /// All recorded costs for `EventKinds`
    pub events: CostsSummary<EventKind>,
    /// All detected performance regressions per callgrind run
    pub regressions: Vec<CallgrindRegressionSummary>,
}

/// TODO: DOCS
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CallgrindTotal {
    pub summary: CostsSummary,
    pub regressions: Vec<CallgrindRegressionSummary>,
}

/// The `CallgrindSummary` summarizes all callgrind runs
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CallgrindSummary {
    /// The paths to the `*.log` files
    pub log_paths: Vec<PathBuf>,
    /// The paths to the `*.out` files
    pub out_paths: Vec<PathBuf>,
    /// The summaries of possibly created flamegraphs
    pub flamegraphs: Vec<FlamegraphSummary>,
    /// The summaries of all callgrind runs
    pub summaries: CallgrindRunSummaries,
}

/// The `CostsDiff` describes the difference between an single optional `new` and `old` cost as
/// percentage and factor.
///
/// There is either a `new`, `old` value present or both. If both values are present, then there is
/// also the `Diffs` present.
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CostsDiff {
    pub costs: EitherOrBoth<u64>,
    pub diffs: Option<Diffs>,
}

/// TODO: DOCS
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum CostsKind {
    #[default]
    None,
    DhatCosts(Costs<DhatMetricKind>),
    ErrorCosts(Costs<ErrorMetricKind>),
    CallgrindCosts(Costs<EventKind>),
}

/// FIX: docs
/// The `CostsSummary` contains all differences for affected [`EventKind`]s
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CostsSummary<K: Hash + Eq = EventKind>(IndexMap<K, CostsDiff>);

/// TODO: DOCS
/// TODO: RENAME (`CostsSummaryByKind`?)
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum CostsSummaryType {
    #[default]
    None,
    ErrorSummary(CostsSummary<ErrorMetricKind>),
    DhatSummary(CostsSummary<DhatMetricKind>),
    CallgrindSummary(CostsSummary<EventKind>),
}

impl CostsSummaryType {
    pub fn add_mut(&mut self, other: &Self) {
        match (self, other) {
            (CostsSummaryType::ErrorSummary(this), CostsSummaryType::ErrorSummary(other)) => {
                this.add(other);
            }
            (CostsSummaryType::DhatSummary(this), CostsSummaryType::DhatSummary(other)) => {
                this.add(other);
            }
            (
                CostsSummaryType::CallgrindSummary(this),
                CostsSummaryType::CallgrindSummary(other),
            ) => {
                this.add(other);
            }
            _ => {}
        }
    }
}

/// TODO: DOCS
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Diffs {
    pub diff_pct: f64,
    pub factor: f64,
}

/// TODO: DOCS
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct FlamegraphSummaries {
    pub summaries: Vec<FlamegraphSummary>,
    pub totals: Vec<FlamegraphSummary>,
}

/// The `FlamegraphSummary` records all created paths for an [`EventKind`] specific flamegraph
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

/// TODO: check DOCS
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ToolRunInfo {
    /// The executed command extracted from Valgrind output
    pub command: String,
    /// The pid of this process
    pub pid: i32,
    /// The parent pid of this process
    pub parent_pid: Option<i32>,
    /// More details for example from the logging output of the tool run
    pub details: Option<String>,
    /// The path to the full logfile from the tool run
    pub path: PathBuf,
    pub part: Option<u64>,
    pub thread: Option<usize>,
}

/// TODO: Make use of it in `ToolSummary`, DOCS
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ToolRunSummaries {
    pub data: Vec<ToolRunSummary>,
    pub total: CostsSummaryType,
}

impl ToolRunSummaries {
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn has_multiple(&self) -> bool {
        self.data.len() > 1
    }
}

/// The `ToolRunSummary` which contains all information about a single tool run process
///
/// There's a separate process and therefore `ToolRunSummary` for the parent process and each child
/// process if `--trace-children=yes` was passed as argument to the `Tool`.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ToolRunSummary {
    pub info: EitherOrBoth<ToolRunInfo>,
    pub costs_summary: CostsSummaryType,
}

/// TODO: There should be a total at least for DHAT
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
    /// All [`ToolRunSummary`]s
    pub summaries: ToolRunSummaries,
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
            version: "2".to_owned(),
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

    pub fn print_and_save(&self, output_format: &OutputFormat) -> Result<()> {
        let value = match (output_format, &self.summary_output) {
            (OutputFormat::Default, None) => return Ok(()),
            _ => {
                serde_json::to_value(self).with_context(|| "Failed to serialize summary to json")?
            }
        };

        let result = match output_format {
            OutputFormat::Default => Ok(()),
            OutputFormat::Json => {
                let output = stdout();
                let writer = output.lock();
                let result = serde_json::to_writer(writer, &value);
                println!();
                result
            }
            OutputFormat::PrettyJson => {
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
    pub fn compare_and_print(&self, id: &str, meta: &Metadata, other: &Self) -> Result<()> {
        if let (Some(callgrind_summary), Some(other_callgrind_summary)) =
            (&self.callgrind_summary, &other.callgrind_summary)
        {
            if let (
                EitherOrBoth::Left(new) | EitherOrBoth::Both(new, _),
                EitherOrBoth::Left(other_new) | EitherOrBoth::Both(other_new, _),
            ) = (
                callgrind_summary.summaries.total.summary.extract_costs(),
                other_callgrind_summary
                    .summaries
                    .total
                    .summary
                    .extract_costs(),
            ) {
                let new_summary = CostsSummary::new(EitherOrBoth::Both(new, other_new));
                VerticalFormat.print_comparison(
                    meta,
                    &self.function_name,
                    id,
                    self.details.as_deref(),
                    &CostsSummaryType::CallgrindSummary(new_summary),
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
            summaries: CallgrindRunSummaries::default(),
        }
    }

    /// Return true if there are any recorded regressions in this `CallgrindSummary`
    pub fn is_regressed(&self) -> bool {
        self.summaries
            .summaries
            .iter()
            .any(|r| !r.regressions.is_empty())
    }

    /// TODO: REMOVE
    /// Create and add a [`CallgrindRunSummary`] to this `CallgrindSummary`
    pub fn add_summary(
        &mut self,
        bench_bin: &Path,
        bench_args: &[OsString],
        old_path: &ToolOutputPath,
        events: CostsSummary,
        regressions: Vec<CallgrindRegressionSummary>,
    ) {
        self.summaries.summaries.push(CallgrindRunSummary {
            command: format!(
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
            ),
            baseline: old_path.exists().then(|| Baseline {
                kind: old_path.baseline_kind.clone(),
                path: old_path.to_path(),
            }),
            events,
            regressions,
        });
    }

    pub fn add_summaries(
        &mut self,
        bench_bin: &Path,
        bench_args: &[OsString],
        // TODO: USE a type Baselines = (Option<Baseline, ...)
        baselines: &(Option<String>, Option<String>),
        summaries: Summaries,
        regressions: Vec<CallgrindRegressionSummary>,
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
        for summary in summaries.data {
            let old_baseline = match summary.details {
                EitherOrBoth::Left(_) => None,
                EitherOrBoth::Both(_, old) | EitherOrBoth::Right(old) => Some(Baseline {
                    kind: baselines.1.as_ref().map_or(BaselineKind::Old, |name| {
                        BaselineKind::Name(BaselineName(name.to_owned()))
                    }),
                    path: old.0,
                }),
            };

            self.summaries.summaries.push(CallgrindRunSummary {
                command: command.clone(),
                baseline: old_baseline,
                events: summary.costs_summary,
                regressions: vec![],
            });
        }

        self.summaries.total.summary = summaries.total.clone();
        self.summaries.total.regressions = regressions;
    }
}

impl CostsDiff {
    pub fn new(costs: EitherOrBoth<u64>) -> Self {
        if let EitherOrBoth::Both(new, old) = costs {
            Self {
                costs,
                diffs: Some(Diffs::new(new, old)),
            }
        } else {
            Self { costs, diffs: None }
        }
    }

    pub fn add(&self, other: &Self) -> Self {
        match (&self.costs, &other.costs) {
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

impl Diffs {
    pub fn new(new: u64, old: u64) -> Self {
        Self {
            diff_pct: percentage_diff(new, old),
            factor: factor_diff(new, old),
        }
    }
}

impl<K> CostsSummary<K>
where
    K: Hash + Eq + Summarize + Display + Clone,
{
    /// TODO: TEST
    /// Create a new `CostsSummary` calculating the differences between new and old (if any)
    /// [`Costs`]
    pub fn new(costs: EitherOrBoth<Costs<K>>) -> Self {
        match costs {
            EitherOrBoth::Left(new) => {
                let mut new = Cow::Owned(new);
                K::summarize(&mut new);

                Self(
                    new.iter()
                        .map(|(event_kind, cost)| {
                            (
                                event_kind.clone(),
                                CostsDiff::new(EitherOrBoth::Left(*cost)),
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
                                CostsDiff::new(EitherOrBoth::Right(*cost)),
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
                for event_kind in new.event_kinds_union(&old) {
                    let diff = match (new.cost_by_kind(&event_kind), old.cost_by_kind(&event_kind))
                    {
                        (Some(cost), None) => CostsDiff::new(EitherOrBoth::Left(cost)),
                        (None, Some(cost)) => CostsDiff::new(EitherOrBoth::Right(cost)),
                        (Some(new), Some(old)) => CostsDiff::new(EitherOrBoth::Both(new, old)),
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
    pub fn diff_by_kind(&self, event_kind: &K) -> Option<&CostsDiff> {
        self.0.get(event_kind)
    }

    pub fn all_diffs(&self) -> impl Iterator<Item = (&K, &CostsDiff)> {
        self.0.iter()
    }

    pub fn extract_costs(&self) -> EitherOrBoth<Costs<K>> {
        let mut new_costs: Costs<K> = Costs::empty();
        let mut old_costs: Costs<K> = Costs::empty();
        // The diffs should not be empty
        for (event_kind, diff) in self.all_diffs() {
            match diff.costs {
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

impl<K> Default for CostsSummary<K>
where
    K: Hash + Eq,
{
    fn default() -> Self {
        Self(IndexMap::default())
    }
}

impl CostsSummaryType {
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

impl ToolRunSummary {
    pub fn new_has_errors(&self) -> bool {
        match &self.costs_summary {
            CostsSummaryType::None
            | CostsSummaryType::DhatSummary(_)
            | CostsSummaryType::CallgrindSummary(_) => false,
            CostsSummaryType::ErrorSummary(costs) => costs
                .diff_by_kind(&ErrorMetricKind::Errors)
                .map_or(false, |e| match e.costs {
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
        let new_diff = CostsDiff::new(cost);
        let old_diff = CostsDiff::new(other_cost);
        let expected = CostsDiff::new(expected);

        assert_eq!(new_diff.add(&old_diff), expected);
        assert_eq!(old_diff.add(&new_diff), expected);
    }
}
