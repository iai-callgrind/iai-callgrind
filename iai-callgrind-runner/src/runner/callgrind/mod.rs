pub mod args;
pub mod flamegraph;
pub mod flamegraph_parser;
pub mod hashmap_parser;
pub mod model;
pub mod parser;
pub mod sentinel_parser;
pub mod summary_parser;

use std::convert::AsRef;
use std::ffi::{OsStr, OsString};
use std::fmt::Display;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::{anyhow, Context, Result};
use colored::{ColoredString, Colorize};
use log::{debug, error, info, Level};

use super::callgrind::args::Args;
use super::meta::Metadata;
use crate::api::ExitWith;
use crate::error::Error;
use crate::util::{
    resolve_binary_path, truncate_str_utf8, write_all_to_stderr, write_all_to_stdout,
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

#[derive(Clone, Debug)]
pub struct CallgrindStats {
    /// Ir: equals the number of instructions executed
    instructions_executed: u64,
    /// I1mr: I1 cache read misses
    l1_instructions_cache_read_misses: u64,
    /// ILmr: LL cache instruction read misses
    l3_instructions_cache_read_misses: u64,
    /// Dr: Memory reads
    total_data_cache_reads: u64,
    /// D1mr: D1 cache read misses
    l1_data_cache_read_misses: u64,
    /// DLmr: LL cache data read misses
    l3_data_cache_read_misses: u64,
    /// Dw: Memory writes
    total_data_cache_writes: u64,
    /// D1mw: D1 cache write misses
    l1_data_cache_write_misses: u64,
    /// DLmw: LL cache data write misses
    l3_data_cache_write_misses: u64,
}

#[derive(Clone, Debug)]
pub struct CallgrindSummary {
    instructions: u64,
    l1_hits: u64,
    l3_hits: u64,
    ram_hits: u64,
    total_memory_rw: u64,
    cycles: u64,
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
        match (output.status.code().unwrap(), exit_with) {
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

impl CallgrindStats {
    fn summarize(&self) -> CallgrindSummary {
        let ram_hits = self.l3_instructions_cache_read_misses
            + self.l3_data_cache_read_misses
            + self.l3_data_cache_write_misses;
        let l1_data_accesses = self.l1_data_cache_read_misses + self.l1_data_cache_write_misses;
        let l1_miss = self.l1_instructions_cache_read_misses + l1_data_accesses;
        let l3_accesses = l1_miss;
        let l3_hits = l3_accesses - ram_hits;

        let total_memory_rw =
            self.instructions_executed + self.total_data_cache_reads + self.total_data_cache_writes;
        let l1_hits = total_memory_rw - ram_hits - l3_hits;

        // Uses Itamar Turner-Trauring's formula from https://pythonspeed.com/articles/consistent-benchmarking-in-ci/
        let cycles = l1_hits + (5 * l3_hits) + (35 * ram_hits);

        CallgrindSummary {
            instructions: self.instructions_executed,
            l1_hits,
            l3_hits,
            ram_hits,
            total_memory_rw,
            cycles,
        }
    }

    fn signed_short(n: f64) -> String {
        let n_abs = n.abs();

        if n_abs < 10.0f64 {
            format!("{n:+.6}")
        } else if n_abs < 100.0f64 {
            format!("{n:+.5}")
        } else if n_abs < 1000.0f64 {
            format!("{n:+.4}")
        } else if n_abs < 10000.0f64 {
            format!("{n:+.3}")
        } else if n_abs < 100_000.0_f64 {
            format!("{n:+.2}")
        } else if n_abs < 1_000_000.0_f64 {
            format!("{n:+.1}")
        } else {
            format!("{n:+.0}")
        }
    }

    fn percentage_diff(new: u64, old: u64) -> ColoredString {
        fn format(string: &ColoredString) -> ColoredString {
            ColoredString::from(format!(" ({string})").as_str())
        }

        if new == old {
            return format(&"No Change".bright_black());
        }

        #[allow(clippy::cast_precision_loss)]
        let new = new as f64;
        #[allow(clippy::cast_precision_loss)]
        let old = old as f64;

        let diff = (new - old) / old;
        let pct = diff * 100.0f64;

        if pct.is_sign_positive() {
            format(
                &format!("{:>+6}%", Self::signed_short(pct))
                    .bright_red()
                    .bold(),
            )
        } else {
            format(
                &format!("{:>+6}%", Self::signed_short(pct))
                    .bright_green()
                    .bold(),
            )
        }
    }

    pub fn print(&self, old: Option<CallgrindStats>) {
        let summary = self.summarize();
        let old_summary = old.map(|stat| stat.summarize());
        println!(
            "  Instructions:     {:>15}{}",
            summary.instructions.to_string().bold(),
            match &old_summary {
                Some(old) => Self::percentage_diff(summary.instructions, old.instructions),
                None => String::new().normal(),
            }
        );
        println!(
            "  L1 Hits:          {:>15}{}",
            summary.l1_hits.to_string().bold(),
            match &old_summary {
                Some(old) => Self::percentage_diff(summary.l1_hits, old.l1_hits),
                None => String::new().normal(),
            }
        );
        println!(
            "  L2 Hits:          {:>15}{}",
            summary.l3_hits.to_string().bold(),
            match &old_summary {
                Some(old) => Self::percentage_diff(summary.l3_hits, old.l3_hits),
                None => String::new().normal(),
            }
        );
        println!(
            "  RAM Hits:         {:>15}{}",
            summary.ram_hits.to_string().bold(),
            match &old_summary {
                Some(old) => Self::percentage_diff(summary.ram_hits, old.ram_hits),
                None => String::new().normal(),
            }
        );
        println!(
            "  Total read+write: {:>15}{}",
            summary.total_memory_rw.to_string().bold(),
            match &old_summary {
                Some(old) => Self::percentage_diff(summary.total_memory_rw, old.total_memory_rw),
                None => String::new().normal(),
            }
        );
        println!(
            "  Estimated Cycles: {:>15}{}",
            summary.cycles.to_string().bold(),
            match &old_summary {
                Some(old) => Self::percentage_diff(summary.cycles, old.cycles),
                None => String::new().normal(),
            }
        );
    }
}
