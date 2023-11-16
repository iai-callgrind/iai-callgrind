use std::borrow::Cow;
use std::ffi::OsString;
use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use glob::glob;
use indexmap::{indexmap, IndexMap};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::callgrind::model::Costs;
use super::tool::{ToolOutputPath, ValgrindTool};
use super::Error;
use crate::api::EventKind;
use crate::util::{factor_diff, make_absolute, percentage_diff};

/// A `Baseline` depending on the [`BaselineKind`] which points to the corresponding path
///
/// This baseline is used for comparisons with the new output of valgrind tools.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct Baseline {
    pub kind: BaselineKind,
    pub path: PathBuf,
}

/// The `BaselineKind` describing the baseline
///
/// Currently, iai-callgrind can only compare callgrind output with `.old` files.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum BaselineKind {
    Old,
}

/// The `BenchmarkKind`
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum BenchmarkKind {
    LibraryBenchmark,
    BinaryBenchmark,
}

/// The `BenchmarkSummary` containing all the information of a single benchmark run
///
/// This includes produced files, recorded callgrind events, performance regressions ...
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct BenchmarkSummary {
    pub version: String,
    pub kind: BenchmarkKind,
    pub summary_output: Option<SummaryOutput>,
    pub project_root: PathBuf,
    pub package_dir: PathBuf,
    pub benchmark_file: PathBuf,
    pub benchmark_exe: PathBuf,
    pub module_path: String,
    pub id: Option<String>,
    pub details: Option<String>,
    pub callgrind_summary: Option<CallgrindSummary>,
    pub tool_summaries: Vec<ToolSummary>,
}

/// The `CallgrindRegressionSummary` describing a single event based performance regression
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CallgrindRegressionSummary {
    pub event_kind: EventKind,
    pub new: u64,
    pub old: u64,
    pub diff_pct: f64,
    pub limit: f64,
}

/// The `CallgrindRunSummary` containing the recorded events, performance regressions of a single
/// callgrind run
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CallgrindRunSummary {
    pub command: String,
    pub baseline: Option<Baseline>,
    pub events: CostsSummary,
    pub regressions: Vec<CallgrindRegressionSummary>,
}

/// The `CallgrindSummary` summarizes all callgrind runs
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CallgrindSummary {
    pub regression_fail_fast: bool,
    pub log_paths: Vec<PathBuf>,
    pub out_paths: Vec<PathBuf>,
    pub flamegraphs: Vec<FlamegraphSummary>,
    pub summaries: Vec<CallgrindRunSummary>,
}

/// The `CostsDiff` describes the difference between an optional `new` and `old` cost as percentage
/// and factor
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CostsDiff {
    pub new: Option<u64>,
    pub old: Option<u64>,
    pub diff_pct: Option<f64>,
    pub factor: Option<f64>,
}

/// The `CostsSummary` contains all differences for affected [`EventKind`]s
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct CostsSummary(IndexMap<EventKind, CostsDiff>);

/// The `FlamegraphSummary` records all created paths for an [`EventKind`] specific flamegraph
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct FlamegraphSummary {
    pub event_kind: EventKind,
    pub regular_path: Option<PathBuf>,
    pub old_path: Option<PathBuf>,
    pub diff_path: Option<PathBuf>,
}

/// The format (json, ...) in which the summary file should be saved or printed
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum SummaryFormat {
    Json,
    PrettyJson,
}

/// Manage the summary output file with this `SummaryOutput`
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct SummaryOutput {
    format: SummaryFormat,
    path: PathBuf,
}

/// The `ToolRunSummary` which contains all information about a single tool run process
///
/// There's a separate process and therefore `ToolRunSummary` for the parent process and each child
/// process
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ToolRunSummary {
    pub command: String,
    pub pid: String,
    pub baseline: Option<Baseline>,
    pub summary: IndexMap<String, String>,
}

/// The `ToolSummary` containing all information about a valgrind tool run
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub struct ToolSummary {
    pub tool: ValgrindTool,
    pub log_paths: Vec<PathBuf>,
    pub out_paths: Vec<PathBuf>,
    pub summaries: Vec<ToolRunSummary>,
}

impl BenchmarkSummary {
    /// Create a new `BenchmarkSummary`
    ///
    /// Relative paths are made absolute with the `project_root` as base directory.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: BenchmarkKind,
        project_root: PathBuf,
        package_dir: PathBuf,
        benchmark_file: PathBuf,
        benchmark_exe: PathBuf,
        segments: &[&str],
        id: Option<String>,
        details: Option<String>,
        output: Option<SummaryOutput>,
    ) -> Self {
        Self {
            version: "1".to_owned(),
            kind,
            benchmark_file: make_absolute(&project_root, benchmark_file),
            benchmark_exe: make_absolute(&project_root, benchmark_exe),
            module_path: segments.join("::"),
            id,
            details,
            callgrind_summary: None,
            tool_summaries: vec![],
            summary_output: output,
            project_root,
            package_dir,
        }
    }

    /// If this `BenchmarkSummary` has a value in the option `SummaryOutput` save it in json format
    pub fn save_json(&self, pretty: bool) -> Result<()> {
        if let Some(output) = &self.summary_output {
            let mut file = output.create()?;
            if pretty {
                serde_json::to_writer_pretty(&mut file, self)
                    .with_context(|| "Failed to serialize to json".to_owned())?;
            } else {
                serde_json::to_writer(&mut file, self)
                    .with_context(|| "Failed to serialize to json".to_owned())?;
            }
        }

        Ok(())
    }

    /// If this `BenchmarkSummary` has a value in the option `SummaryOutput` save it
    pub fn save(&self) -> Result<()> {
        if let Some(output) = &self.summary_output {
            match output.format {
                SummaryFormat::Json => self.save_json(false)?,
                SummaryFormat::PrettyJson => self.save_json(true)?,
            }
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
    pub fn check_regression(&self, is_regressed: &mut bool) -> Result<()> {
        if let Some(callgrind_summary) = &self.callgrind_summary {
            let benchmark_is_regressed = callgrind_summary.is_regressed();
            if benchmark_is_regressed && callgrind_summary.regression_fail_fast {
                return Err(Error::RegressionError(true).into());
            }

            *is_regressed |= benchmark_is_regressed;
        }

        Ok(())
    }
}

impl CallgrindSummary {
    /// Create a new `CallgrindSummary`
    pub fn new(
        fail_fast: bool,
        log_paths: Vec<PathBuf>,
        out_paths: Vec<PathBuf>,
    ) -> CallgrindSummary {
        Self {
            regression_fail_fast: fail_fast,
            log_paths,
            out_paths,
            flamegraphs: Vec::default(),
            summaries: Vec::default(),
        }
    }

    /// Return true if there are any recorded regressions in this `CallgrindSummary`
    pub fn is_regressed(&self) -> bool {
        self.summaries.iter().any(|r| !r.regressions.is_empty())
    }

    /// Create and add a [`CallgrindRunSummary`] to this `CallgrindSummary`
    pub fn add_summary(
        &mut self,
        bench_bin: &Path,
        bench_args: &[OsString],
        old_output: &ToolOutputPath,
        events: CostsSummary,
        regressions: Vec<CallgrindRegressionSummary>,
    ) {
        self.summaries.push(CallgrindRunSummary {
            command: format!(
                "{} {}",
                bench_bin.display(),
                shlex::join(
                    bench_args
                        .iter()
                        .map(|s| s.to_string_lossy().to_string())
                        .collect::<Vec<String>>()
                        .as_slice()
                        .iter()
                        .map(std::string::String::as_str)
                )
            ),
            baseline: old_output.exists().then(|| Baseline {
                kind: BaselineKind::Old,
                path: old_output.to_path(),
            }),
            events,
            regressions,
        });
    }
}

impl CostsSummary {
    /// Create a new `CostsSummary` calculating the differences between new and old (if any)
    /// [`Costs`]
    pub fn new(new_costs: &Costs, old_costs: Option<&Costs>) -> Self {
        let mut new_costs = Cow::Borrowed(new_costs);
        if !new_costs.is_summarized() {
            _ = new_costs.to_mut().make_summary();
        }

        if let Some(old_costs) = old_costs {
            let mut old_costs = Cow::Borrowed(old_costs);
            if !old_costs.is_summarized() {
                _ = old_costs.to_mut().make_summary();
            }
            let mut map = indexmap! {};
            for event_kind in new_costs.event_kinds_union(old_costs.as_ref()) {
                let diff = match (
                    new_costs.cost_by_kind(&event_kind),
                    old_costs.cost_by_kind(&event_kind),
                ) {
                    (None, Some(cost)) => CostsDiff {
                        new: None,
                        old: Some(cost),
                        diff_pct: None,
                        factor: None,
                    },
                    (Some(cost), None) => CostsDiff {
                        new: Some(cost),
                        old: None,
                        diff_pct: None,
                        factor: None,
                    },
                    (Some(new), Some(old)) => CostsDiff {
                        new: Some(new),
                        old: Some(old),
                        diff_pct: Some(percentage_diff(new, old)),
                        factor: Some(factor_diff(new, old)),
                    },
                    (None, None) => unreachable!(),
                };
                map.insert(event_kind, diff);
            }
            Self(map)
        } else {
            CostsSummary(
                new_costs
                    .iter()
                    .map(|(event_kind, cost)| {
                        (
                            *event_kind,
                            CostsDiff {
                                new: Some(*cost),
                                old: None,
                                diff_pct: None,
                                factor: None,
                            },
                        )
                    })
                    .collect::<IndexMap<_, _>>(),
            )
        }
    }
}

impl FlamegraphSummary {
    /// Create a new `FlamegraphSummary`
    pub fn new(event_kind: EventKind) -> Self {
        Self {
            event_kind,
            regular_path: Option::default(),
            old_path: Option::default(),
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
    pub fn init(&self) {
        for entry in glob(self.path.with_extension("*").to_string_lossy().as_ref())
            .expect("Glob pattern should be valid")
        {
            std::fs::remove_file(entry.unwrap().as_path())
                .expect("Path from matched glob pattern should be present");
        }
    }

    /// Try to create an empty summary file returning the [`File`] object
    pub fn create(&self) -> Result<File> {
        File::create(&self.path).with_context(|| "Failed to create json summary file")
    }
}
