use std::collections::VecDeque;
use std::ffi::OsString;
use std::fmt::Display;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use colored::{ColoredString, Colorize};
use log::{debug, error, info, log_enabled, trace, warn, Level};
use which::which;

use crate::api::{ExitWith, RawCallgrindArgs};
use crate::error::{IaiCallgrindError, Result};
use crate::util::{
    bool_to_yesno, truncate_str_utf8, write_all_to_stderr, write_all_to_stdout, yesno_to_bool,
};

#[derive(Debug, Default, Clone)]
pub struct CallgrindOptions {
    pub env_clear: bool,
    pub current_dir: Option<PathBuf>,
    pub entry_point: Option<String>,
    pub exit_with: Option<ExitWith>,
    pub envs: Vec<(OsString, OsString)>,
}

pub struct CallgrindCommand {
    command: Command,
}

impl CallgrindCommand {
    pub fn new(allow_aslr: bool, arch: &str) -> Self {
        // Invoke Valgrind, disabling ASLR if possible because ASLR could noise up the results a bit
        let command = if allow_aslr {
            debug!("Running with ASLR enabled");
            Command::new("valgrind")
        } else if cfg!(target_os = "linux") {
            debug!("Running with ASLR disabled");
            let mut cmd = Command::new("setarch");
            cmd.args([arch, "-R", "valgrind"]);
            cmd
        } else if cfg!(target_os = "freebsd") {
            debug!("Running with ASLR disabled");
            let mut cmd = Command::new("proccontrol");
            cmd.args(["-m", "aslr", "-s", "disable", "valgrind"]);
            cmd
        } else {
            debug!("Could not switch ASLR off. Running with ASLR enabled");
            Command::new("valgrind")
        };

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
                Err(IaiCallgrindError::BenchmarkLaunchError(output))
            }
            (0i32, Some(ExitWith::Failure)) => {
                error!(
                    "Expected benchmark '{}' to fail but it succeeded",
                    executable.display(),
                );
                Err(IaiCallgrindError::BenchmarkLaunchError(output))
            }
            (_, Some(ExitWith::Failure)) => Ok((output.stdout, output.stderr)),
            (code, Some(ExitWith::Success)) => {
                error!(
                    "Expected benchmark '{}' to succeed but it exited with '{}'",
                    executable.display(),
                    code
                );
                Err(IaiCallgrindError::BenchmarkLaunchError(output))
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
                Err(IaiCallgrindError::BenchmarkLaunchError(output))
            }
            _ => Err(IaiCallgrindError::BenchmarkLaunchError(output)),
        }
    }

    pub fn run(
        self,
        mut callgrind_args: CallgrindArgs,
        executable: &Path,
        executable_args: &[OsString],
        options: CallgrindOptions,
        output_file: &Path,
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
        callgrind_args.set_output_file(output_file);

        let callgrind_args = callgrind_args.to_vec();
        debug!("Callgrind arguments: {}", &callgrind_args.join(" "));

        let executable = if executable.is_absolute() {
            executable.to_owned()
        } else {
            let e = which(executable).map_err(|error| {
                IaiCallgrindError::Other(format!("{}: '{}'", error, executable.display()))
            })?;
            debug!(
                "Found command '{}' in the PATH: '{}'",
                executable.display(),
                e.display()
            );
            e
        };

        let (stdout, stderr) = command
            .arg("--tool=callgrind")
            .args(callgrind_args)
            .arg(&executable)
            .args(executable_args)
            .envs(envs)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| IaiCallgrindError::LaunchError(PathBuf::from("valgrind"), error))
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

pub struct Sentinel(String);

impl Sentinel {
    pub fn new(value: &str) -> Self {
        Self(format!("fn={value}"))
    }

    pub fn from_path(module: &str, function: &str) -> Self {
        Self(format!("fn={module}::{function}"))
    }

    #[allow(unused)]
    pub fn from_segments<I, T>(segments: T) -> Self
    where
        I: AsRef<str>,
        T: AsRef<[I]>,
    {
        let joined = if let Some((first, suffix)) = segments.as_ref().split_first() {
            suffix.iter().fold(first.as_ref().to_owned(), |mut a, b| {
                a.push_str("::");
                a.push_str(b.as_ref());
                a
            })
        } else {
            String::new()
        };
        Self(format!("fn={joined}"))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Display for Sentinel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<Sentinel> for Sentinel {
    fn as_ref(&self) -> &Sentinel {
        self
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PositionsMode {
    Instr,
    Line,
    InstrLine,
}

impl PositionsMode {
    pub fn from_positions_line(line: &str) -> Option<Self> {
        match line.trim().strip_prefix("positions: ") {
            Some("instr line" | "line instr") => Some(Self::InstrLine),
            Some("instr") => Some(Self::Instr),
            Some("line") => Some(Self::Line),
            Some(_) | None => None,
        }
    }
}

pub struct CallgrindOutput {
    pub file: PathBuf,
}

impl CallgrindOutput {
    pub fn create(base_dir: &Path, module: &str, name: &str) -> Self {
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
            truncate_str_utf8(&sanitized_name, 237) /* callgrind. + .out.old = 18 with max
                                                     * length 255 */
        ));

        let file = current.join(base_dir).join(module_path).join(file_name);
        let output = Self { file };

        std::fs::create_dir_all(output.file.parent().unwrap()).expect("Failed to create directory");

        if output.file.exists() {
            let old_output = output.old_output();
            // Already run this benchmark once; move last results to .old
            std::fs::copy(&output.file, old_output.file).unwrap();
        }

        output
    }

    pub fn exists(&self) -> bool {
        self.file.exists()
    }

    pub fn old_output(&self) -> Self {
        CallgrindOutput {
            file: self.file.with_extension("out.old"),
        }
    }

    pub fn parse_summary(&self) -> CallgrindStats {
        trace!(
            "Parsing callgrind output file '{}' for a summary or totals",
            self.file.display(),
        );

        let file = File::open(&self.file).expect("Unable to open callgrind output file");
        let mut iter = BufReader::new(file)
            .lines()
            .map(std::result::Result::unwrap);
        if !iter
            .by_ref()
            .find(|l| !l.trim().is_empty())
            .expect("Found empty file")
            .contains("callgrind format")
        {
            warn!("Missing file format specifier. Assuming callgrind format.");
        };

        // Ir Dr Dw I1mr D1mr D1mw ILmr DLmr DLmw
        let mut counters: [u64; 9] = [0, 0, 0, 0, 0, 0, 0, 0, 0];
        for line in iter {
            if line.starts_with("summary:") {
                trace!("Found line with summary: '{}'", line);
                for (index, counter) in line
                    .strip_prefix("summary:")
                    .unwrap()
                    .trim()
                    .split_ascii_whitespace()
                    .map(|s| s.parse::<u64>().expect("Encountered non ascii digit"))
                    // we're only interested in the counters for instructions and the cache
                    .take(9)
                    .enumerate()
                {
                    counters[index] += counter;
                }
                trace!("Updated counters to '{:?}'", &counters);
                break;
            }
            if line.starts_with("totals:") {
                trace!("Found line with totals: '{}'", line);
                for (index, counter) in line
                    .strip_prefix("totals:")
                    .unwrap()
                    .trim()
                    .split_ascii_whitespace()
                    .map(|s| s.parse::<u64>().expect("Encountered non ascii digit"))
                    // we're only interested in the counters for instructions and the cache
                    .take(9)
                    .enumerate()
                {
                    counters[index] += counter;
                }
                trace!("Updated counters to '{:?}'", &counters);
                break;
            }
        }

        CallgrindStats {
            instructions_executed: counters[0],
            total_data_cache_reads: counters[1],
            total_data_cache_writes: counters[2],
            l1_instructions_cache_read_misses: counters[3],
            l1_data_cache_read_misses: counters[4],
            l1_data_cache_write_misses: counters[5],
            l3_instructions_cache_read_misses: counters[6],
            l3_data_cache_read_misses: counters[7],
            l3_data_cache_write_misses: counters[8],
        }
    }

    pub fn parse<T>(&self, bench_file: &Path, sentinel: T) -> CallgrindStats
    where
        T: AsRef<Sentinel>,
    {
        let sentinel = sentinel.as_ref();
        trace!(
            "Parsing callgrind output file '{}' for '{}'",
            self.file.display(),
            sentinel
        );

        trace!(
            "Using sentinel: '{}' for file name ending with: '{}'",
            &sentinel,
            bench_file.display()
        );

        let file = File::open(&self.file).expect("Unable to open callgrind output file");
        let mut iter = BufReader::new(file)
            .lines()
            .map(std::result::Result::unwrap);
        if !iter
            .by_ref()
            .find(|l| !l.trim().is_empty())
            .expect("Found empty file")
            .contains("callgrind format")
        {
            warn!("Missing file format specifier. Assuming callgrind format.");
        };

        let mode = iter
            .find_map(|line| PositionsMode::from_positions_line(&line))
            .expect("Callgrind output line with mode for positions");
        trace!("Using parsing mode: {:?}", mode);

        // Ir Dr Dw I1mr D1mr D1mw ILmr DLmr DLmw
        let mut counters: [u64; 9] = [0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut start_record = false;
        for line in iter {
            let line = line.trim_start();
            if line.is_empty() {
                start_record = false;
            }
            if !start_record {
                if line.starts_with("fl=") && line.ends_with(bench_file.to_str().unwrap()) {
                    trace!("Found line with benchmark file: '{}'", line);
                } else if line.starts_with(sentinel.as_str()) {
                    trace!("Found line with sentinel: '{}'", line);
                    start_record = true;
                } else {
                    // do nothing
                }
                continue;
            }

            // we check if it is a line with counters and summarize them
            if line.starts_with(|c: char| c.is_ascii_digit()) {
                // From the documentation of the callgrind format:
                // > If a cost line specifies less event counts than given in the "events" line, the
                // > rest is assumed to be zero.
                trace!("Found line with counters: '{}'", line);
                for (index, counter) in line
                    .split_ascii_whitespace()
                    // skip the first number which is just the line number or instr number or in
                    // case of `instr line` skip 2
                    .skip(if mode == PositionsMode::InstrLine { 2 } else { 1 })
                    .map(|s| s.parse::<u64>().expect("Encountered non ascii digit"))
                    // we're only interested in the counters for instructions and the cache
                    .take(9)
                    .enumerate()
                {
                    counters[index] += counter;
                }
                trace!("Updated counters to '{:?}'", &counters);
            } else {
                trace!("Skipping line: '{}'", line);
            }
        }

        CallgrindStats {
            instructions_executed: counters[0],
            total_data_cache_reads: counters[1],
            total_data_cache_writes: counters[2],
            l1_instructions_cache_read_misses: counters[3],
            l1_data_cache_read_misses: counters[4],
            l1_data_cache_write_misses: counters[5],
            l3_instructions_cache_read_misses: counters[6],
            l3_data_cache_read_misses: counters[7],
            l3_data_cache_write_misses: counters[8],
        }
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct CallgrindArgs {
    i1: String,
    d1: String,
    ll: String,
    cache_sim: bool,
    pub(crate) collect_atstart: bool,
    other: Vec<String>,
    toggle_collect: VecDeque<String>,
    compress_strings: bool,
    compress_pos: bool,
    pub(crate) verbose: bool,
    dump_instr: bool,
    dump_line: bool,
    combine_dumps: bool,
    callgrind_out_file: Option<PathBuf>,
}

impl Default for CallgrindArgs {
    fn default() -> Self {
        Self {
            // Set some reasonable cache sizes. The exact sizes matter less than having fixed sizes,
            // since otherwise callgrind would take them from the CPU and make benchmark runs
            // even more incomparable between machines.
            i1: String::from("32768,8,64"),
            d1: String::from("32768,8,64"),
            ll: String::from("8388608,16,64"),
            cache_sim: true,
            collect_atstart: false,
            compress_pos: false,
            compress_strings: false,
            combine_dumps: true,
            verbose: log_enabled!(log::Level::Debug),
            dump_line: true,
            dump_instr: false,
            toggle_collect: VecDeque::default(),
            callgrind_out_file: Option::default(),
            other: Vec::default(),
        }
    }
}

impl CallgrindArgs {
    pub fn from_raw_callgrind_args(args: &RawCallgrindArgs) -> Self {
        let mut default = Self::default();
        for arg in &args.0 {
            match arg.strip_prefix("--").and_then(|s| s.split_once('=')) {
                Some(("I1", value)) => default.i1 = value.to_owned(),
                Some(("D1", value)) => default.d1 = value.to_owned(),
                Some(("LL", value)) => default.ll = value.to_owned(),
                Some(("collect-atstart", value)) => default.collect_atstart = yesno_to_bool(value),
                Some(("dump-instr", value)) => {
                    default.dump_instr = yesno_to_bool(value);
                }
                Some(("dump-line", value)) => {
                    default.dump_line = yesno_to_bool(value);
                }
                Some(("compress-pos", value)) => default.compress_pos = yesno_to_bool(value),
                Some(("toggle-collect", value)) => {
                    default.toggle_collect.push_back(value.to_owned());
                }
                Some((
                    key @ ("separate-threads" | "cache-sim" | "callgrind-out-file"
                    | "compress-strings" | "combine-dumps"),
                    value,
                )) => {
                    warn!("Ignoring callgrind argument: '--{}={}'", key, value);
                }
                Some(_) => default.other.push(arg.clone()),
                None if arg == "--verbose" => default.verbose = true,
                // ignore positional arguments for now. It may be a filtering argument for cargo
                // bench
                None => {}
            }
        }
        default
    }

    pub fn insert_toggle_collect(&mut self, arg: &str) {
        self.toggle_collect.push_front(arg.to_owned());
    }

    pub fn set_output_file<T>(&mut self, arg: T)
    where
        T: AsRef<Path>,
    {
        self.callgrind_out_file = Some(arg.as_ref().to_owned());
    }

    pub fn to_vec(&self) -> Vec<String> {
        let mut args = vec![
            format!("--I1={}", &self.i1),
            format!("--D1={}", &self.d1),
            format!("--LL={}", &self.ll),
            format!("--cache-sim={}", bool_to_yesno(self.cache_sim)),
            format!("--collect-atstart={}", bool_to_yesno(self.collect_atstart)),
            format!(
                "--compress-strings={}",
                bool_to_yesno(self.compress_strings)
            ),
            format!("--compress-pos={}", bool_to_yesno(self.compress_pos)),
            format!("--dump-line={}", bool_to_yesno(self.dump_line)),
            format!("--dump-instr={}", bool_to_yesno(self.dump_instr)),
            format!("--combine-dumps={}", bool_to_yesno(self.combine_dumps)),
        ];

        if self.verbose {
            args.push(String::from("--verbose"));
        }

        args.append(
            &mut self
                .toggle_collect
                .iter()
                .map(|s| format!("--toggle-collect={s}"))
                .collect::<Vec<String>>(),
        );

        if let Some(output_file) = &self.callgrind_out_file {
            args.push(format!(
                "--callgrind-out-file={}",
                output_file.to_string_lossy(),
            ));
        }

        args.extend_from_slice(self.other.as_slice());
        args
    }
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
