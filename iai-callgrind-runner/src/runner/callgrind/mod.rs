pub mod args;
pub mod flamegraph;
pub mod flamegraph_parser;
pub mod hashmap_parser;
pub mod model;
pub mod parser;
pub mod sentinel_parser;
pub mod summary_parser;

use std::borrow::Cow;
use std::convert::AsRef;
use std::ffi::{OsStr, OsString};
use std::fmt::Display;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use log::{debug, error, info, Level};

use self::model::Costs;
use super::callgrind::args::Args;
use super::meta::Metadata;
use crate::api::{self, EventKind, ExitWith, RegressionConfig};
use crate::error::Error;
use crate::util::{
    resolve_binary_path, to_string_signed_short, truncate_str_utf8, write_all_to_stderr,
    write_all_to_stdout,
};

pub struct CallgrindCommand {
    command: Command,
}

#[derive(Debug, Default, Clone)]
pub struct CallgrindOptions {
    pub env_clear: bool,
    pub current_dir: Option<PathBuf>,
    pub entry_point: Option<String>,
    pub exit_with: Option<ExitWith>,
    pub envs: Vec<(OsString, OsString)>,
}

#[derive(Debug, Clone)]
pub struct CallgrindOutput(PathBuf);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallgrindStats(pub Costs);

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
        let command = meta.valgrind_wrapper.as_ref().map_or_else(
            || {
                let meta_cmd = &meta.valgrind;
                let mut cmd = Command::new(&meta_cmd.bin);
                cmd.args(&meta_cmd.args);
                cmd
            },
            |meta_cmd| {
                let mut cmd = Command::new(&meta_cmd.bin);
                cmd.args(&meta_cmd.args);
                cmd
            },
        );
        Self { command }
    }

    fn check_exit(
        executable: &Path,
        output: Output,
        exit_with: Option<&ExitWith>,
    ) -> Result<(Vec<u8>, Vec<u8>)> {
        let status_code = if let Some(code) = output.status.code() {
            code
        } else {
            return Err(Error::BenchmarkLaunchError(output).into());
        };

        match (status_code, exit_with) {
            (0i32, None | Some(ExitWith::Code(0i32) | ExitWith::Success)) => {
                Ok((output.stdout, output.stderr))
            }
            (0i32, Some(ExitWith::Code(code))) => {
                error!(
                    "Expected benchmark '{}' to exit with '{}' but it succeeded",
                    executable.display(),
                    code
                );
                Err(Error::BenchmarkLaunchError(output).into())
            }
            (0i32, Some(ExitWith::Failure)) => {
                error!(
                    "Expected benchmark '{}' to fail but it succeeded",
                    executable.display(),
                );
                Err(Error::BenchmarkLaunchError(output).into())
            }
            (_, Some(ExitWith::Failure)) => Ok((output.stdout, output.stderr)),
            (code, Some(ExitWith::Success)) => {
                error!(
                    "Expected benchmark '{}' to succeed but it exited with '{}'",
                    executable.display(),
                    code
                );
                Err(Error::BenchmarkLaunchError(output).into())
            }
            (actual_code, Some(ExitWith::Code(expected_code))) if actual_code == *expected_code => {
                Ok((output.stdout, output.stderr))
            }
            (actual_code, Some(ExitWith::Code(expected_code))) => {
                error!(
                    "Expected benchmark '{}' to exit with '{}' but it exited with '{}'",
                    executable.display(),
                    expected_code,
                    actual_code
                );
                Err(Error::BenchmarkLaunchError(output).into())
            }
            _ => Err(Error::BenchmarkLaunchError(output).into()),
        }
    }

    pub fn run(
        self,
        mut callgrind_args: Args,
        executable: &Path,
        executable_args: &[OsString],
        options: CallgrindOptions,
        output: &CallgrindOutput,
    ) -> Result<()> {
        let mut command = self.command;
        debug!(
            "Running callgrind with executable '{}'",
            executable.display()
        );
        let CallgrindOptions {
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
        callgrind_args.set_output_file(&output.0);

        let callgrind_args = callgrind_args.to_vec();
        debug!("Callgrind arguments: {}", &callgrind_args.join(" "));

        let executable = resolve_binary_path(executable)?;

        let (stdout, stderr) = command
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
            .and_then(|output| Self::check_exit(&executable, output, exit_with.as_ref()))?;

        if !stdout.is_empty() {
            info!("Callgrind output on stdout:");
            if log::log_enabled!(Level::Info) {
                write_all_to_stdout(&stdout);
            }
        }
        if !stderr.is_empty() {
            info!("Callgrind output on stderr:");
            if log::log_enabled!(Level::Info) {
                write_all_to_stderr(&stderr);
            }
        }

        Ok(())
    }
}

impl CallgrindOutput {
    pub fn from_existing<T>(path: T) -> Result<Self>
    where
        T: Into<PathBuf>,
    {
        let path: PathBuf = path.into();
        if !path.is_file() {
            return Err(anyhow!(
                "The callgrind output file '{}' did not exist or is not a valid file",
                path.display()
            ));
        }
        Ok(Self(path))
    }

    /// Initialize and create the output directory and organize files
    ///
    /// This method moves the old output to `callgrind.*.out.old`
    pub fn init(base_dir: &Path, module: &str, name: &str) -> Self {
        let current = base_dir;
        let module_path: PathBuf = module.split("::").collect();
        let sanitized_name = sanitize_filename::sanitize_with_options(
            name,
            sanitize_filename::Options {
                windows: false,
                truncate: false,
                replacement: "_",
            },
        );
        let file_name = PathBuf::from(format!(
            "callgrind.{}.out",
            // callgrind. + .out.old = 18 + 37 bytes headroom for extensions with more than 3
            // bytes. max length is usually 255 bytes
            truncate_str_utf8(&sanitized_name, 200)
        ));

        let path = current.join(base_dir).join(module_path).join(file_name);
        let output = Self(path);

        std::fs::create_dir_all(output.0.parent().unwrap()).expect("Failed to create directory");

        if output.exists() {
            let old_output = output.to_old_output();
            // Already run this benchmark once; move last results to .old
            std::fs::copy(&output.0, old_output.0).unwrap();
        }

        output
    }

    pub fn exists(&self) -> bool {
        self.0.exists()
    }

    pub fn with_extension<T>(&self, extension: T) -> Self
    where
        T: AsRef<OsStr>,
    {
        Self(self.0.with_extension(extension))
    }

    pub fn to_old_output(&self) -> Self {
        Self(self.0.with_extension("out.old"))
    }

    pub fn open(&self) -> Result<File> {
        File::open(&self.0)
            .with_context(|| format!("Error opening callgrind output file '{}'", self.0.display()))
    }

    pub fn lines(&self) -> Result<impl Iterator<Item = String>> {
        let file = self.open()?;
        Ok(BufReader::new(file)
            .lines()
            .map(std::result::Result::unwrap))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl Display for CallgrindOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0.display()))
    }
}

impl Regression {
    /// Check regression of the [`CallgrindStats`] for the configured [`EventKind`]s and print it
    ///
    /// If the old `CallgrindStats` is None then no regression checks are performed and this method
    /// returns [`Ok`].
    ///
    /// # Errors
    ///
    /// Returns an [`anyhow::Error`] with the only source [`Error::RegressionError`] if a regression
    /// error occurred
    pub fn check_and_print(
        &self,
        new: &CallgrindStats,
        old: Option<&CallgrindStats>,
    ) -> Result<()> {
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

    fn check(
        &self,
        new: &CallgrindStats,
        old: Option<&CallgrindStats>,
    ) -> Vec<(EventKind, u64, u64, f64, f64)> {
        let mut regressions = vec![];
        if let Some(old) = old {
            let mut new_costs = Cow::Borrowed(&new.0);
            let mut old_costs = Cow::Borrowed(&old.0);

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
                    #[allow(clippy::cast_precision_loss)]
                    let new_cost_float = new_cost as f64;
                    #[allow(clippy::cast_precision_loss)]
                    let old_cost_float = old_cost as f64;

                    let diff = (new_cost_float - old_cost_float) / old_cost_float;
                    let pct = diff * 100.0f64;
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
        let new = CallgrindStats(cachesim_costs([0, 0, 0, 0, 0, 0, 0, 0, 0]));
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

        let new = CallgrindStats(cachesim_costs(new));
        let old = Some(CallgrindStats(cachesim_costs(old)));

        assert_eq!(regression.check(&new, old.as_ref()), expected);
    }
}
