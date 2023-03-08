use cfg_if::cfg_if;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

mod macros;

/// A function that is opaque to the optimizer, used to prevent the compiler from
/// optimizing away computations in a benchmark.
///
/// This variant is stable-compatible, but it may cause some performance overhead
/// or fail to prevent code from being eliminated.
pub fn black_box<T>(dummy: T) -> T {
    unsafe {
        let ret = std::ptr::read_volatile(&dummy);
        std::mem::forget(dummy);
        ret
    }
}

fn check_valgrind() -> bool {
    let result = Command::new("valgrind")
        .arg("--tool=callgrind")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match result {
        Err(e) => {
            println!("Unexpected error while launching valgrind. Error: {}", e);
            false
        }
        Ok(status) => {
            if status.success() {
                true
            } else {
                println!("Failed to launch valgrind. Error: {}. Please ensure that valgrind is installed and on the $PATH.", status);
                false
            }
        }
    }
}

fn get_arch() -> String {
    let output = Command::new("uname")
        .arg("-m")
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to run `uname` to determine CPU architecture.");

    String::from_utf8(output.stdout)
        .expect("`-uname -m` returned invalid unicode.")
        .trim()
        .to_owned()
}

fn basic_valgrind() -> Command {
    Command::new("valgrind")
}

// Invoke Valgrind, disabling ASLR if possible because ASLR could noise up the results a bit
cfg_if! {
    if #[cfg(target_os = "linux")] {
        fn valgrind_without_aslr(arch: &str) -> Command {
            let mut cmd = Command::new("setarch");
            cmd.arg(arch)
                .arg("-R")
                .arg("valgrind");
            cmd
        }
    } else if #[cfg(target_os = "freebsd")] {
        fn valgrind_without_aslr(_arch: &str) -> Command {
            let mut cmd = Command::new("proccontrol");
            cmd.arg("-m")
                .arg("aslr")
                .arg("-s")
                .arg("disable");
            cmd
        }
    } else {
        fn valgrind_without_aslr(_arch: &str) -> Command {
            // Can't disable ASLR on this platform
            basic_valgrind()
        }
    }
}

fn run_bench(
    module: &str,
    arch: &str,
    executable: &str,
    index: usize,
    name: &str,
    allow_aslr: bool,
) -> (CallgrindStats, Option<CallgrindStats>) {
    let target = PathBuf::from("target/iai");
    let module_path: PathBuf = module.split("::").collect();
    let file_name = PathBuf::from(format!("callgrind.{}.out", name));

    let mut output_file = target;
    output_file.push(module_path);
    output_file.push(file_name);
    let old_file = output_file.with_extension("out.old");
    std::fs::create_dir_all(output_file.parent().unwrap()).expect("Failed to create directory");

    if output_file.exists() {
        // Already run this benchmark once; move last results to .old
        std::fs::copy(&output_file, &old_file).unwrap();
    }

    let mut cmd = if allow_aslr {
        basic_valgrind()
    } else {
        valgrind_without_aslr(arch)
    };

    let status = cmd
        // Set some reasonable cache sizes. The exact sizes matter less than having fixed sizes,
        // since otherwise callgrind would take them from the CPU and make benchmark runs
        // even more incomparable between machines.
        .arg("--tool=callgrind")
        .arg("--I1=32768,8,64")
        .arg("--D1=32768,8,64")
        .arg("--LL=8388608,16,64")
        .arg("--cache-sim=yes")
        .arg("--collect-atstart=no")
        .arg(format!("--toggle-collect=*{}::{}", module, name))
        .arg("--compress-strings=no")
        .arg("--compress-pos=no")
        .arg(format!("--callgrind-out-file={}", output_file.display()))
        .arg(executable)
        .arg("--iai-run")
        .arg(index.to_string())
        // Currently not used in iai-callgrind itself, but in `callgrind_annotate` this name is
        // shown and makes it easier to identify the benchmark under test
        .arg(format!("{}::{}", module, name))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("Failed to run benchmark in ");

    if !status.success() {
        panic!(
            "Failed to run benchmark in callgrind. Exit code: {}",
            status
        );
    }

    let new_stats = parse_callgrind_output(&output_file);
    let old_stats = if old_file.exists() {
        Some(parse_callgrind_output(&old_file))
    } else {
        None
    };

    (new_stats, old_stats)
}

fn parse_callgrind_output(file: &Path) -> CallgrindStats {
    let mut events_line = None;
    let mut summary_line = None;

    let file_in = File::open(file).expect("Unable to open callgrind output file");

    for line in BufReader::new(file_in).lines() {
        let line = line.unwrap();
        if let Some(line) = line.strip_prefix("events: ") {
            events_line = Some(line.trim().to_owned());
        }
        if let Some(line) = line.strip_prefix("summary: ") {
            summary_line = Some(line.trim().to_owned());
        }
    }

    match (events_line, summary_line) {
        (Some(events), Some(summary)) => {
            let events: HashMap<_, _> = events
                .split_whitespace()
                .zip(summary.split_whitespace().map(|s| {
                    s.parse::<u64>()
                        .expect("Unable to parse summary line from callgrind output file")
                }))
                .collect();

            CallgrindStats {
                instruction_reads: *events.get("Ir").unwrap_or(&0),
                instruction_l1_misses: *events.get("I1mr").unwrap_or(&0),
                instruction_cache_misses: *events.get("ILmr").unwrap_or(&0),
                data_reads: *events.get("Dr").unwrap_or(&0),
                data_l1_read_misses: *events.get("D1mr").unwrap_or(&0),
                data_cache_read_misses: *events.get("DLmr").unwrap_or(&0),
                data_writes: *events.get("Dw").unwrap_or(&0),
                data_l1_write_misses: *events.get("D1mw").unwrap_or(&0),
                data_cache_write_misses: *events.get("DLmw").unwrap_or(&0),
            }
        }
        _ => panic!("Unable to parse callgrind output file"),
    }
}

#[derive(Clone, Debug)]
struct CallgrindStats {
    instruction_reads: u64,
    instruction_l1_misses: u64,
    instruction_cache_misses: u64,
    data_reads: u64,
    data_l1_read_misses: u64,
    data_cache_read_misses: u64,
    data_writes: u64,
    data_l1_write_misses: u64,
    data_cache_write_misses: u64,
}
impl CallgrindStats {
    pub fn ram_accesses(&self) -> u64 {
        self.instruction_cache_misses + self.data_cache_read_misses + self.data_cache_write_misses
    }
    pub fn summarize(&self) -> CallgrindSummary {
        let ram_hits = self.ram_accesses();
        let l3_accesses =
            self.instruction_l1_misses + self.data_l1_read_misses + self.data_l1_write_misses;
        let l3_hits = l3_accesses - ram_hits;

        let total_memory_rw = self.instruction_reads + self.data_reads + self.data_writes;
        let l1_hits = total_memory_rw - (ram_hits + l3_hits);

        CallgrindSummary {
            l1_hits,
            l3_hits,
            ram_hits,
        }
    }
}

#[derive(Clone, Debug)]
struct CallgrindSummary {
    l1_hits: u64,
    l3_hits: u64,
    ram_hits: u64,
}
impl CallgrindSummary {
    fn cycles(&self) -> u64 {
        // Uses Itamar Turner-Trauring's formula from https://pythonspeed.com/articles/consistent-benchmarking-in-ci/
        self.l1_hits + (5 * self.l3_hits) + (35 * self.ram_hits)
    }
}

/// Custom-test-framework runner. Should not be called directly.
#[doc(hidden)]
#[inline(never)]
pub fn runner(
    module: &str,
    executable: &str,
    is_iai_run: bool,
    benches: &[&(&'static str, fn())],
    current_index: Option<usize>,
) {
    if is_iai_run {
        // In this branch, we're running under callgrind, so execute the benchmark as quickly as
        // possible and exit
        benches[current_index.unwrap()].1();
        return;
    }

    // Otherwise we're running normally, under cargo
    if !check_valgrind() {
        return;
    }

    let arch = get_arch();

    let allow_aslr = std::env::var_os("IAI_ALLOW_ASLR").is_some();

    for (index, (name, _func)) in benches.iter().enumerate() {
        println!("{}", name);
        let (stats, old_stats) = run_bench(module, &arch, executable, index, name, allow_aslr);

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

        fn percentage_diff(new: u64, old: u64) -> String {
            if new == old {
                return " (No change)".to_owned();
            }

            let new: f64 = new as f64;
            let old: f64 = old as f64;

            let diff = (new - old) / old;
            let pct = diff * 100.0;

            format!(" ({:>+6}%)", signed_short(pct))
        }

        println!(
            "  Instructions:     {:>15}{}",
            stats.instruction_reads,
            match &old_stats {
                Some(old) => percentage_diff(stats.instruction_reads, old.instruction_reads),
                None => String::new(),
            }
        );
        let summary = stats.summarize();
        let old_summary = old_stats.map(|stat| stat.summarize());
        println!(
            "  L1 Accesses:      {:>15}{}",
            summary.l1_hits,
            match &old_summary {
                Some(old) => percentage_diff(summary.l1_hits, old.l1_hits),
                None => String::new(),
            }
        );
        println!(
            "  L2 Accesses:      {:>15}{}",
            summary.l3_hits,
            match &old_summary {
                Some(old) => percentage_diff(summary.l3_hits, old.l3_hits),
                None => String::new(),
            }
        );
        println!(
            "  RAM Accesses:     {:>15}{}",
            summary.ram_hits,
            match &old_summary {
                Some(old) => percentage_diff(summary.ram_hits, old.ram_hits),
                None => String::new(),
            }
        );
        println!(
            "  Estimated Cycles: {:>15}{}",
            summary.cycles(),
            match &old_summary {
                Some(old) => percentage_diff(summary.cycles(), old.cycles()),
                None => String::new(),
            }
        );
        println!();
    }
}
