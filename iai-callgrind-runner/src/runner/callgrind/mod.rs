pub mod args;
pub mod flamegraph;
pub mod flamegraph_parser;
pub mod hashmap_parser;
pub mod model;
pub mod parser;
pub mod sentinel_parser;
pub mod summary_parser;

use std::borrow::Cow;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::Result;
use colored::Colorize;
use log::debug;

use self::model::Costs;
use super::callgrind::args::Args;
use super::common::ToolOutputPath;
use super::meta::Metadata;
use super::tool::RunOptions;
use crate::api::{self, EventKind, RegressionConfig};
use crate::error::Error;
use crate::runner::common::ValgrindTool;
use crate::runner::tool::{check_exit, ToolOutput};
use crate::util::{percentage_diff, resolve_binary_path, to_string_signed_short};

pub struct CallgrindCommand {
    command: Command,
}

#[derive(Clone, Debug)]
pub struct CallgrindSummary {
    l1_hits: u64,
    l3_hits: u64,
    ram_hits: u64,
    total_memory_rw: u64,
    cycles: u64,
}

#[derive(Debug, Clone)]
pub struct Regression {
    pub limits: Vec<(EventKind, f64)>,
    pub fail_fast: bool,
}

impl CallgrindCommand {
    pub fn new(meta: &Metadata) -> Self {
        Self {
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
        callgrind_args.set_output_file(&output_path.path);

        let callgrind_args = callgrind_args.to_vec();
        debug!("Callgrind arguments: {}", &callgrind_args.join(" "));

        let executable = resolve_binary_path(executable)?;

        let output = command
            .arg("--tool=callgrind")
            .args(callgrind_args)
            .arg(&executable)
            .args(executable_args)
            .envs(envs)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| {
                Error::LaunchError(PathBuf::from("valgrind"), error.to_string()).into()
            })
            .and_then(|output| check_exit(&executable, output, exit_with.as_ref()))?;

        Ok(ToolOutput {
            tool: ValgrindTool::Callgrind,
            output,
        })
    }
}

impl TryFrom<&Costs> for CallgrindSummary {
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

impl Regression {
    /// Check regression of the [`Costs`] for the configured [`EventKind`]s and print it
    ///
    /// If the old `Costs` is None then no regression checks are performed and this method returns
    /// [`Ok`].
    ///
    /// # Errors
    ///
    /// Returns an [`anyhow::Error`] with the only source [`Error::RegressionError`] if a regression
    /// error occurred
    pub fn check_and_print(&self, new: &Costs, old: Option<&Costs>) -> Result<()> {
        let regressions = self.check(new, old);
        if regressions.is_empty() {
            return Ok(());
        }
        for (event_kind, new_cost, old_cost, pct, limit) in regressions {
            if limit.is_sign_positive() {
                println!(
                    "Performance has {0}: {1} ({new_cost} > {old_cost}) regressed by {2:>+6} \
                     (>{3:>+6})",
                    "regressed".bold().bright_red(),
                    event_kind.to_string().bold(),
                    format!("{}%", to_string_signed_short(pct))
                        .bold()
                        .bright_red(),
                    to_string_signed_short(limit).bright_black()
                );
            } else {
                println!(
                    "Performance has {0}: {1} ({new_cost} < {old_cost}) regressed by {2:>+6} \
                     (<{3:>+6})",
                    "regressed".bold().bright_red(),
                    event_kind.to_string().bold(),
                    format!("{}%", to_string_signed_short(pct))
                        .bold()
                        .bright_red(),
                    to_string_signed_short(limit).bright_black()
                );
            }
        }

        Err(Error::RegressionError(self.fail_fast).into())
    }

    fn check(&self, new: &Costs, old: Option<&Costs>) -> Vec<(EventKind, u64, u64, f64, f64)> {
        let mut regressions = vec![];
        if let Some(old) = old {
            let mut new_costs = Cow::Borrowed(new);
            let mut old_costs = Cow::Borrowed(old);

            for (event_kind, limit) in &self.limits {
                if event_kind.is_derived() {
                    if !new_costs.is_summarized() {
                        _ = new_costs.to_mut().make_summary();
                    }
                    if !old_costs.is_summarized() {
                        _ = old_costs.to_mut().make_summary();
                    }
                }

                if let (Some(new_cost), Some(old_cost)) = (
                    new_costs.cost_by_kind(event_kind),
                    old_costs.cost_by_kind(event_kind),
                ) {
                    let pct = percentage_diff(new_cost, old_cost);
                    if limit.is_sign_positive() {
                        if pct > *limit {
                            regressions.push((*event_kind, new_cost, old_cost, pct, *limit));
                        }
                    } else if pct < *limit {
                        regressions.push((*event_kind, new_cost, old_cost, pct, *limit));
                    } else {
                        // no regression
                    }
                }
            }
        }
        regressions
    }
}

impl From<api::RegressionConfig> for Regression {
    fn from(value: api::RegressionConfig) -> Self {
        let RegressionConfig { limits, fail_fast } = value;
        Regression {
            limits: if limits.is_empty() {
                vec![(EventKind::EstimatedCycles, 10f64)]
            } else {
                limits
            },
            fail_fast: fail_fast.unwrap_or(false),
        }
    }
}

impl Default for Regression {
    fn default() -> Self {
        Self {
            limits: vec![(EventKind::EstimatedCycles, 10f64)],
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
        let regression = Regression::default();
        let new = cachesim_costs([0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let old = None;

        assert!(regression.check(&new, old).is_empty());
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
        let regression = Regression {
            limits,
            ..Default::default()
        };

        let new = cachesim_costs(new);
        let old = Some(cachesim_costs(old));

        assert_eq!(regression.check(&new, old.as_ref()), expected);
    }
}
