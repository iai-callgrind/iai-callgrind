use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use cfg_if::cfg_if;
use colored::{ColoredString, Colorize};
use iai_callgrind::Options;
use log::{debug, info, trace, warn};

use crate::util::{
    bool_to_yesno, concat_os_string, join_os_string, write_all_to_stderr, write_all_to_stdout,
    yesno_to_bool,
};
use crate::IaiCallgrindError;

// Invoke Valgrind, disabling ASLR if possible because ASLR could noise up the results a bit
cfg_if! {
    if #[cfg(target_os = "linux")] {
        pub fn valgrind_without_aslr(arch: &str) -> Option<Command> {
            let mut cmd = Command::new("setarch");
            cmd.arg(arch)
                .arg("-R")
                .arg("valgrind");
            Some(cmd)
        }
    } else if #[cfg(target_os = "freebsd")] {
        pub fn valgrind_without_aslr(_arch: &str) -> Option<Command> {
            let mut cmd = Command::new("proccontrol");
            cmd.arg("-m")
                .arg("aslr")
                .arg("-s")
                .arg("disable");
            Some(cmd)
        }
    } else {
        pub fn valgrind_without_aslr(_arch: &str) -> Option<Command> {
            // Can't disable ASLR on this platform
            None
        }
    }
}

pub struct CallgrindCommand {
    command: Command,
}

impl CallgrindCommand {
    pub fn new(allow_aslr: bool, arch: &str) -> Self {
        let command = if allow_aslr {
            debug!("Running with ASLR enabled");
            Command::new("valgrind")
        } else {
            match valgrind_without_aslr(arch) {
                Some(cmd) => {
                    debug!("Running with ASLR disabled");
                    cmd
                }
                None => {
                    debug!("Running with ASLR enabled");
                    Command::new("valgrind")
                }
            }
        };
        Self { command }
    }

    pub fn run(
        self,
        callgrind_args: &CallgrindArgs,
        executable: &Path,
        executable_args: Vec<String>,
        envs: Vec<(String, String)>,
        options: &Options,
    ) -> Result<(), IaiCallgrindError> {
        let mut command = self.command;
        debug!(
            "Running callgrind with executable '{}'",
            executable.display()
        );
        let Options {
            env_clear,
            current_dir,
            entry_point: _,
        } = options;

        if *env_clear {
            debug!("Clearing environment variables");
            command.env_clear();
        }
        if let Some(dir) = current_dir {
            debug!("Setting current directory to '{}'", dir.display());
            command.current_dir(dir);
        }

        let callgrind_args = callgrind_args.to_os_args();
        debug!(
            "Callgrind arguments: {}",
            join_os_string(&callgrind_args, OsStr::new(" ")).to_string_lossy()
        );
        let (stdout, stderr) = command
            .arg("--tool=callgrind")
            .args(callgrind_args)
            .arg(executable)
            .args(executable_args)
            .envs(envs)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(IaiCallgrindError::LaunchError)
            .and_then(|output| {
                if output.status.success() {
                    Ok((output.stdout, output.stderr))
                } else {
                    Err(IaiCallgrindError::CallgrindLaunchError(output))
                }
            })?;

        if !stdout.is_empty() {
            info!("Callgrind output on stdout:");
            write_all_to_stdout(&stdout);
        }
        if !stderr.is_empty() {
            info!("Callgrind output on stderr:");
            write_all_to_stderr(&stderr)
        }

        Ok(())
    }
}

pub struct CallgrindOutput {
    pub file: PathBuf,
}

impl CallgrindOutput {
    pub fn create(module: &str, name: &str) -> Self {
        let current = std::env::current_dir().unwrap();
        let target = PathBuf::from("target/iai");
        let module_path: PathBuf = module.split("::").collect();
        let file_name = PathBuf::from(format!("callgrind.{}.out", name));

        let file = current.join(target).join(module_path).join(file_name);
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

        let file_in = File::open(&self.file).expect("Unable to open callgrind output file");
        let mut iter = BufReader::new(file_in).lines().map(|l| l.unwrap());
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
            l1_instructions_cache_reads: counters[0],
            total_data_cache_reads: counters[1],
            total_data_cache_writes: counters[2],
            l1_instructions_cache_read_misses: counters[3],
            l1_data_cache_read_misses: counters[4],
            l1_data_cache_write_misses: counters[5],
            l3_instructions_cache_misses: counters[6],
            l3_data_cache_read_misses: counters[7],
            l3_data_cache_write_misses: counters[8],
        }
    }

    pub fn parse(&self, bench_file: &Path, module: &str, function_name: &str) -> CallgrindStats {
        trace!(
            "Parsing callgrind output file '{}' for '{}::{}'",
            self.file.display(),
            module,
            function_name
        );

        let sentinel = format!("fn={}", [module, function_name].join("::"));
        trace!(
            "Using sentinel: '{}' for file name ending with: '{}'",
            &sentinel,
            bench_file.display()
        );

        let file_in = File::open(&self.file).expect("Unable to open callgrind output file");
        let mut iter = BufReader::new(file_in).lines().map(|l| l.unwrap());
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
        let mut start_record = false;
        let mut maybe_counting = false;
        let mut start_counting = false;
        for line in iter {
            let line = line.trim_start();
            if line.is_empty() {
                start_record = false;
                maybe_counting = false;
                start_counting = false;
            }
            if !start_record {
                if line.starts_with("fl=") && line.ends_with(bench_file.to_str().unwrap()) {
                    trace!("Found line with benchmark file: '{}'", line);
                } else if line.starts_with(&sentinel) {
                    trace!("Found line with sentinel: '{}'", line);
                    start_record = true;
                }
                continue;
            }
            // We're only interested in the counters for event counters within the benchmark
            // function and ignore counters for the benchmark function itself.
            if !maybe_counting {
                if line.starts_with("cfn=") {
                    trace!("Found line with a calling function: '{}'", line);
                    maybe_counting = true;
                }
                continue;
            }
            if !start_counting {
                if line.starts_with("calls") {
                    trace!("Found line with calls: '{}'. Starting the counting", line);
                    start_counting = true;
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
                    // skip the first number which is just the line number
                    .skip(1)
                    .map(|s| s.parse::<u64>().expect("Encountered non ascii digit"))
                    // we're only interested in the counters for instructions and the cache
                    .take(9)
                    .enumerate()
                {
                    counters[index] += counter;
                }
                trace!("Updated counters to '{:?}'", &counters);
            } else if line.starts_with("cfn=") {
                trace!("Found line with a calling function: '{}'", line);
                start_counting = false;
            } else {
                trace!("Pausing counting. End of a cfn record");
                maybe_counting = false;
                start_counting = false;
            }
        }

        CallgrindStats {
            l1_instructions_cache_reads: counters[0],
            total_data_cache_reads: counters[1],
            total_data_cache_writes: counters[2],
            l1_instructions_cache_read_misses: counters[3],
            l1_data_cache_read_misses: counters[4],
            l1_data_cache_write_misses: counters[5],
            l3_instructions_cache_misses: counters[6],
            l3_data_cache_read_misses: counters[7],
            l3_data_cache_write_misses: counters[8],
        }
    }
}
#[derive(Debug, Clone)]
pub struct CallgrindArgs {
    i1: String,
    d1: String,
    ll: String,
    cache_sim: bool,
    pub collect_atstart: bool,
    other: Vec<OsString>,
    toggle_collect: VecDeque<String>,
    compress_strings: bool,
    compress_pos: bool,
    // TODO: Should be a PathBuf
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
            toggle_collect: Default::default(),
            compress_pos: false,
            compress_strings: false,
            callgrind_out_file: Default::default(),
            other: Default::default(),
        }
    }
}

impl CallgrindArgs {
    pub fn from_args(args: &[OsString]) -> Self {
        let mut default = Self::default();
        for arg in args.iter() {
            let string = arg.to_string_lossy();
            match string.strip_prefix("--").and_then(|s| s.split_once('=')) {
                Some(("I1", value)) => default.i1 = value.to_owned(),
                Some(("D1", value)) => default.d1 = value.to_owned(),
                Some(("LL", value)) => default.ll = value.to_owned(),
                Some(("collect-atstart", value)) => default.collect_atstart = yesno_to_bool(value),
                Some(("compress-strings", value)) => {
                    default.compress_strings = yesno_to_bool(value)
                }
                Some(("compress-pos", value)) => default.compress_pos = yesno_to_bool(value),
                Some(("toggle-collect", value)) => {
                    default.toggle_collect.push_back(value.to_owned())
                }
                Some(("cache-sim", value)) => {
                    warn!("Ignoring callgrind argument: '--cache-sim={}'", value)
                }
                Some(("callgrind-out-file", value)) => warn!(
                    "Ignoring callgrind argument: '--callgrind-out-file={}'",
                    value
                ),
                Some(_) => default.other.push(arg.to_owned()),
                None => panic!("Error parsing callgrind arguments"),
            }
        }
        default
    }

    pub fn insert_toggle_collect(&mut self, arg: &str) -> &mut Self {
        self.toggle_collect.push_front(arg.to_owned());
        self
    }

    pub fn set_output_file(&mut self, arg: &str) -> &mut Self {
        self.callgrind_out_file = Some(PathBuf::from(arg));
        self
    }

    pub fn to_os_args(&self) -> Vec<OsString> {
        let mut args = vec![
            concat_os_string(OsString::from("--I1="), &self.i1),
            concat_os_string(OsString::from("--D1="), &self.d1),
            concat_os_string(OsString::from("--LL="), &self.ll),
            concat_os_string(
                OsString::from("--cache-sim="),
                bool_to_yesno(self.cache_sim),
            ),
            concat_os_string(
                OsString::from("--collect-atstart="),
                bool_to_yesno(self.collect_atstart),
            ),
            concat_os_string(
                OsString::from("--compress-strings="),
                bool_to_yesno(self.compress_strings),
            ),
            concat_os_string(
                OsString::from("--compress-pos="),
                bool_to_yesno(self.compress_pos),
            ),
        ];

        args.append(
            &mut self
                .toggle_collect
                .iter()
                .map(|s| concat_os_string(OsString::from("--toggle-collect="), s))
                .collect::<Vec<OsString>>(),
        );

        if let Some(output_file) = &self.callgrind_out_file {
            args.push(concat_os_string(
                OsString::from("--callgrind-out-file="),
                output_file.as_os_str(),
            ));
        }

        args.extend_from_slice(self.other.as_slice());
        args
    }
}

#[derive(Clone, Debug)]
pub struct CallgrindSummary {
    l1_instructions: u64,
    l1_data_hits: u64,
    l3_hits: u64,
    ram_hits: u64,
    total_memory_rw: u64,
    cycles: u64,
}

#[derive(Clone, Debug)]
pub struct CallgrindStats {
    /// Ir: equals the number of instructions executed
    l1_instructions_cache_reads: u64,
    /// I1mr: I1 cache read misses
    l1_instructions_cache_read_misses: u64,
    /// ILmr: LL cache instruction read misses
    l3_instructions_cache_misses: u64,
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
        let ram_hits = self.l3_instructions_cache_misses
            + self.l3_data_cache_read_misses
            + self.l3_data_cache_write_misses;
        let l1_data_accesses = self.l1_data_cache_read_misses + self.l1_data_cache_write_misses;
        let l1_miss = self.l1_instructions_cache_read_misses + l1_data_accesses;
        let l3_accesses = l1_miss;
        let l3_hits = l3_accesses - ram_hits;

        let total_memory_rw = self.l1_instructions_cache_reads
            + self.total_data_cache_reads
            + self.total_data_cache_writes;
        let l1_data_hits =
            total_memory_rw - self.l1_instructions_cache_reads - (ram_hits + l3_hits);
        assert!(
            total_memory_rw == l1_data_hits + self.l1_instructions_cache_reads + l3_hits + ram_hits
        );

        // Uses Itamar Turner-Trauring's formula from https://pythonspeed.com/articles/consistent-benchmarking-in-ci/
        let cycles =
            self.l1_instructions_cache_reads + l1_data_hits + (5 * l3_hits) + (35 * ram_hits);

        CallgrindSummary {
            l1_instructions: self.l1_instructions_cache_reads,
            l1_data_hits,
            l3_hits,
            ram_hits,
            total_memory_rw,
            cycles,
        }
    }

    fn signed_short(n: f64) -> String {
        let n_abs = n.abs();

        if n_abs < 10.0 {
            format!("{:+.6}", n)
        } else if n_abs < 100.0 {
            format!("{:+.5}", n)
        } else if n_abs < 1000.0 {
            format!("{:+.4}", n)
        } else if n_abs < 10000.0 {
            format!("{:+.3}", n)
        } else if n_abs < 100000.0 {
            format!("{:+.2}", n)
        } else if n_abs < 1000000.0 {
            format!("{:+.1}", n)
        } else {
            format!("{:+.0}", n)
        }
    }

    fn percentage_diff(new: u64, old: u64) -> ColoredString {
        fn format(string: ColoredString) -> ColoredString {
            ColoredString::from(format!(" ({})", string).as_str())
        }

        if new == old {
            return format("No Change".bright_black());
        }

        let new = new as f64;
        let old = old as f64;

        let diff = (new - old) / old;
        let pct = diff * 100.0;

        if pct.is_sign_positive() {
            format(
                format!("{:>+6}%", Self::signed_short(pct))
                    .bright_red()
                    .bold(),
            )
        } else {
            format(
                format!("{:>+6}%", Self::signed_short(pct))
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
            summary.l1_instructions.to_string().bold(),
            match &old_summary {
                Some(old) => Self::percentage_diff(summary.l1_instructions, old.l1_instructions),
                None => String::new().normal(),
            }
        );
        println!(
            "  L1 Data Hits:     {:>15}{}",
            summary.l1_data_hits.to_string().bold(),
            match &old_summary {
                Some(old) => Self::percentage_diff(summary.l1_data_hits, old.l1_data_hits),
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
