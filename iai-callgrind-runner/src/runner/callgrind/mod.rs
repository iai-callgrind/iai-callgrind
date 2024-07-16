pub mod args;
pub mod flamegraph;
pub mod flamegraph_parser;
pub mod hashmap_parser;
pub mod model;
pub mod parser;
pub mod sentinel_parser;
pub mod summary_parser;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::Result;
use colored::Colorize;
use log::debug;

use self::args::Args;
use self::model::Costs;
use super::args::NoCapture;
use super::meta::Metadata;
use super::summary::{CallgrindRegressionSummary, CostsSummary};
use super::tool::{check_exit, RunOptions, ToolOutput, ToolOutputPath, ValgrindTool};
use crate::api::{self, EventKind};
use crate::error::Error;
use crate::runner::format;
use crate::util::{resolve_binary_path, to_string_signed_short};

pub struct CallgrindCommand {
    command: Command,
    nocapture: NoCapture,
}

#[derive(Clone, Debug)]
pub struct CacheSummary {
    l1_hits: u64,
    l3_hits: u64,
    ram_hits: u64,
    total_memory_rw: u64,
    cycles: u64,
}

#[derive(Debug, Clone)]
pub struct RegressionConfig {
    pub limits: Vec<(EventKind, f64)>,
    pub fail_fast: bool,
}

impl CallgrindCommand {
    pub fn new(meta: &Metadata) -> Self {
        Self {
            nocapture: meta.args.nocapture,
            command: meta.into(),
        }
    }

    pub fn run(
        self,
        mut callgrind_args: Args,
        executable: &Path,
        executable_args: &[OsString],
        options: RunOptions,
        output_path: &ToolOutputPath,
    ) -> Result<ToolOutput> {
        let mut command = self.command;
        debug!(
            "Running callgrind with executable '{}'",
            executable.display()
        );
        let RunOptions {
            env_clear,
            current_dir,
            exit_with,
            entry_point,
            envs,
        } = options;

        if env_clear {
            debug!("Clearing environment variables");
            command.env_clear();
        }
        if let Some(dir) = current_dir {
            debug!("Setting current directory to '{}'", dir.display());
            command.current_dir(dir);
        }

        if let Some(entry_point) = entry_point {
            callgrind_args.collect_atstart = false;
            callgrind_args.insert_toggle_collect(&entry_point);
        } else {
            callgrind_args.collect_atstart = true;
        }
        callgrind_args.set_output_file(&output_path.to_path());
        callgrind_args.set_log_arg(output_path);

        let callgrind_args = callgrind_args.to_vec();
        debug!("Callgrind arguments: {}", &callgrind_args.join(" "));

        let executable = resolve_binary_path(executable)?;

        command
            .arg("--tool=callgrind")
            .args(callgrind_args)
            .arg(&executable)
            .args(executable_args)
            .envs(envs);

        match self.nocapture {
            NoCapture::True | NoCapture::False => {}
            NoCapture::Stderr => {
                command.stdout(Stdio::null()).stderr(Stdio::inherit());
            }
            NoCapture::Stdout => {
                command.stdout(Stdio::inherit()).stderr(Stdio::null());
            }
        };

        let output = match self.nocapture {
            NoCapture::False => command
                .output()
                .map_err(|error| {
                    Error::LaunchError(PathBuf::from("valgrind"), error.to_string()).into()
                })
                .and_then(|output| {
                    let status = output.status;
                    check_exit(
                        ValgrindTool::Callgrind,
                        &executable,
                        Some(output),
                        status,
                        &output_path.to_log_output(),
                        exit_with.as_ref(),
                    )
                })?,
            NoCapture::True | NoCapture::Stderr | NoCapture::Stdout => {
                command
                    .status()
                    .map_err(|error| {
                        Error::LaunchError(PathBuf::from("valgrind"), error.to_string()).into()
                    })
                    .and_then(|status| {
                        check_exit(
                            ValgrindTool::Callgrind,
                            &executable,
                            None,
                            status,
                            &output_path.to_log_output(),
                            exit_with.as_ref(),
                        )
                    })?;
                println!("{}", format::no_capture_footer(self.nocapture));
                None
            }
        };

        Ok(ToolOutput {
            tool: ValgrindTool::Callgrind,
            output,
        })
    }
}

impl TryFrom<&Costs> for CacheSummary {
    type Error = anyhow::Error;

    fn try_from(value: &Costs) -> std::result::Result<Self, Self::Error> {
        use EventKind::*;
        //         0   1  2    3    4    5    6    7    8
        // events: Ir Dr Dw I1mr D1mr D1mw ILmr DLmr DLmw
        let instructions = value.try_cost_by_kind(&Ir)?;
        let total_data_cache_reads = value.try_cost_by_kind(&Dr)?;
        let total_data_cache_writes = value.try_cost_by_kind(&Dw)?;
        let l1_instructions_cache_read_misses = value.try_cost_by_kind(&I1mr)?;
        let l1_data_cache_read_misses = value.try_cost_by_kind(&D1mr)?;
        let l1_data_cache_write_misses = value.try_cost_by_kind(&D1mw)?;
        let l3_instructions_cache_read_misses = value.try_cost_by_kind(&ILmr)?;
        let l3_data_cache_read_misses = value.try_cost_by_kind(&DLmr)?;
        let l3_data_cache_write_misses = value.try_cost_by_kind(&DLmw)?;

        let ram_hits = l3_instructions_cache_read_misses
            + l3_data_cache_read_misses
            + l3_data_cache_write_misses;
        let l1_data_accesses = l1_data_cache_read_misses + l1_data_cache_write_misses;
        let l1_miss = l1_instructions_cache_read_misses + l1_data_accesses;
        let l3_accesses = l1_miss;
        let l3_hits = l3_accesses - ram_hits;

        let total_memory_rw = instructions + total_data_cache_reads + total_data_cache_writes;
        let l1_hits = total_memory_rw - ram_hits - l3_hits;

        // Uses Itamar Turner-Trauring's formula from https://pythonspeed.com/articles/consistent-benchmarking-in-ci/
        let cycles = l1_hits + (5 * l3_hits) + (35 * ram_hits);

        Ok(Self {
            l1_hits,
            l3_hits,
            ram_hits,
            total_memory_rw,
            cycles,
        })
    }
}

impl RegressionConfig {
    /// Check regression of the [`Costs`] for the configured [`EventKind`]s and print it
    ///
    /// If the old `Costs` is None then no regression checks are performed and this method returns
    /// [`Ok`].
    ///
    /// # Errors
    ///
    /// Returns an [`anyhow::Error`] with the only source [`Error::RegressionError`] if a regression
    /// error occurred
    pub fn check_and_print(&self, costs_summary: &CostsSummary) -> Vec<CallgrindRegressionSummary> {
        let regression_summaries = self.check(costs_summary);

        for CallgrindRegressionSummary {
            event_kind,
            new,
            old,
            diff_pct,
            limit,
        } in &regression_summaries
        {
            if limit.is_sign_positive() {
                eprintln!(
                    "Performance has {0}: {1} ({new} > {old}) regressed by {2:>+6} (>{3:>+6})",
                    "regressed".bold().bright_red(),
                    event_kind.to_string().bold(),
                    format!("{}%", to_string_signed_short(*diff_pct))
                        .bold()
                        .bright_red(),
                    to_string_signed_short(*limit).bright_black()
                );
            } else {
                eprintln!(
                    "Performance has {0}: {1} ({new} < {old}) regressed by {2:>+6} (<{3:>+6})",
                    "regressed".bold().bright_red(),
                    event_kind.to_string().bold(),
                    format!("{}%", to_string_signed_short(*diff_pct))
                        .bold()
                        .bright_red(),
                    to_string_signed_short(*limit).bright_black()
                );
            }
        }

        regression_summaries
    }

    pub fn check(&self, costs_summary: &CostsSummary) -> Vec<CallgrindRegressionSummary> {
        let mut regressions = vec![];
        for (event_kind, limit) in &self.limits {
            if let Some((new_cost, old_cost, pct)) = costs_summary
                .diff_by_kind(event_kind)
                .filter(|d| d.diff_pct.is_some())
                // These unwraps are safe since if diff_pct is present new and old are also present
                .map(|d| (d.new.unwrap(), d.old.unwrap(), d.diff_pct.unwrap()))
            {
                if limit.is_sign_positive() {
                    if pct > *limit {
                        let summary = CallgrindRegressionSummary {
                            event_kind: *event_kind,
                            new: new_cost,
                            old: old_cost,
                            diff_pct: pct,
                            limit: *limit,
                        };
                        regressions.push(summary);
                    }
                } else if pct < *limit {
                    let summary = CallgrindRegressionSummary {
                        event_kind: *event_kind,
                        new: new_cost,
                        old: old_cost,
                        diff_pct: pct,
                        limit: *limit,
                    };
                    regressions.push(summary);
                } else {
                    // no regression
                }
            }
        }
        regressions
    }
}

impl From<api::RegressionConfig> for RegressionConfig {
    fn from(value: api::RegressionConfig) -> Self {
        let api::RegressionConfig { limits, fail_fast } = value;
        RegressionConfig {
            limits: if limits.is_empty() {
                vec![(EventKind::Ir, 10f64)]
            } else {
                limits
            },
            fail_fast: fail_fast.unwrap_or(false),
        }
    }
}

impl Default for RegressionConfig {
    fn default() -> Self {
        Self {
            limits: vec![(EventKind::Ir, 10f64)],
            fail_fast: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use EventKind::*;

    use super::*;

    fn cachesim_costs(costs: [u64; 9]) -> Costs {
        Costs::with_event_kinds([
            (Ir, costs[0]),
            (Dr, costs[1]),
            (Dw, costs[2]),
            (I1mr, costs[3]),
            (D1mr, costs[4]),
            (D1mw, costs[5]),
            (ILmr, costs[6]),
            (DLmr, costs[7]),
            (DLmw, costs[8]),
        ])
    }

    #[rstest]
    fn test_regression_check_when_old_is_none() {
        let regression = RegressionConfig::default();
        let new = cachesim_costs([0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let old = None;
        let summary = CostsSummary::new(&new, old);

        assert!(regression.check(&summary).is_empty());
    }

    #[rstest]
    #[case::ir_all_zero(
        vec![(Ir, 0f64)],
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![]
    )]
    #[case::ir_when_regression(
        vec![(Ir, 0f64)],
        [2, 0, 0, 0, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![(Ir, 2, 1, 100f64, 0f64)]
    )]
    #[case::ir_when_improved(
        vec![(Ir, 0f64)],
        [1, 0, 0, 0, 0, 0, 0, 0, 0],
        [2, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![]
    )]
    #[case::ir_when_negative_limit(
        vec![(Ir, -49f64)],
        [1, 0, 0, 0, 0, 0, 0, 0, 0],
        [2, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![(Ir, 1, 2, -50f64, -49f64)]
    )]
    #[case::derived_all_zero(
        vec![(EstimatedCycles, 0f64)],
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![]
    )]
    #[case::derived_when_regression(
        vec![(EstimatedCycles, 0f64)],
        [2, 0, 0, 0, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![(EstimatedCycles, 2, 1, 100f64, 0f64)]
    )]
    #[case::derived_when_regression_multiple(
        vec![(EstimatedCycles, 5f64), (Ir, 10f64)],
        [2, 0, 0, 0, 0, 0, 0, 0, 0],
        [1, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![(EstimatedCycles, 2, 1, 100f64, 5f64), (Ir, 2, 1, 100f64, 10f64)]
    )]
    #[case::derived_when_improved(
        vec![(EstimatedCycles, 0f64)],
        [1, 0, 0, 0, 0, 0, 0, 0, 0],
        [2, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![]
    )]
    #[case::derived_when_regression_mixed(
        vec![(EstimatedCycles, 0f64)],
        [96, 24, 18, 6, 0, 2, 6, 0, 2],
        [48, 12, 9, 3, 0, 1, 3, 0, 1],
        vec![(EstimatedCycles, 410, 205, 100f64, 0f64)]
    )]
    fn test_regression_check_when_old_is_some(
        #[case] limits: Vec<(EventKind, f64)>,
        #[case] new: [u64; 9],
        #[case] old: [u64; 9],
        #[case] expected: Vec<(EventKind, u64, u64, f64, f64)>,
    ) {
        let regression = RegressionConfig {
            limits,
            ..Default::default()
        };

        let new = cachesim_costs(new);
        let old = Some(cachesim_costs(old));
        let summary = CostsSummary::new(&new, old.as_ref());
        let expected = expected
            .iter()
            .map(|(e, n, o, d, l)| CallgrindRegressionSummary {
                event_kind: *e,
                new: *n,
                old: *o,
                diff_pct: *d,
                limit: *l,
            })
            .collect::<Vec<CallgrindRegressionSummary>>();

        assert_eq!(regression.check(&summary), expected);
    }
}
