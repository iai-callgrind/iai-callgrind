// spell-checker:ignore rmdirs
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::fs::File;
use std::io::{stderr, stdout, BufRead, Read, Write as IOWrite};
use std::os::unix::process::ExitStatusExt;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use benchmark_tests::common::Summary;
use benchmark_tests::serde::runs_on::RunsOn;
use colored::Colorize;
use fs_extra::dir::CopyOptions;
use glob::glob;
use lazy_static::lazy_static;
use minijinja::Environment;
use once_cell::sync::OnceCell;
use regex::{Captures, Regex};
use rustc_version::{Channel, VersionMeta};
use serde::{Deserialize, Serialize};
use tempfile::{tempdir, TempDir};
use valico::json_schema;
use valico::json_schema::schema::ScopedSchema;

const PACKAGE: &str = "benchmark-tests";
const TEMPLATE_BENCH_NAME: &str = "test_bench_template";
const TEMPLATE_CONTENT: &str = r#"fn main() {
    panic!("should be replaced by a rendered template");
}
"#;
const SCHEMA_PATH: &str = "iai-callgrind-runner/schemas";
const SCHEMA_VERSION: &str = "4";

static TEMPLATE_DATA: OnceCell<HashMap<String, minijinja::Value>> = OnceCell::new();

// The regex patterns working on the `stdout` must not include the indentation. The indentation can
// be different depending on the `show_grid` option and starts either with 2 spaces (`  `) or if
// `show_grid` is `true` with a pipe character (`|`)
lazy_static! {
    static ref NUMBERS_RE: Regex = Regex::new(
        r"(?x)
            (?<desc>.+:\s*)(?<comp1>[0-9.]+|N/A)\|(?<comp2>[0-9.]+|N/A)
            (?<diff>
                (?<diff_percent>(?<white1>\s*)(?<percent>\(.*\)))
                (?<diff_factor>(?<white2>\s*)(?<factor>\[.*\]))?
            )?"
    )
    .expect("Regex should compile");
    static ref RUNNING_RE: Regex = Regex::new(r"^[ ]+Running .*$").expect("Regex should compile");
    static ref PROCESS_DID_NOT_EXIT_SUCCESSFULLY_RE: Regex =
        Regex::new(r"^([ ]+process didn't exit successfully: `)(.*)(` \(exit status: .*\).*)$")
            .expect("Regex should compile");
    // Performance has regressed: Instructions (133 -> 196) regressed by +47.3684% (>+0.00000%)
    // $1<__NUM__>$3<__NUM__>$5<__PERCENT__>$7<__NUM__>$9
    static ref REGRESSION_RE: Regex =
        Regex::new(r"^(Performance has regressed:\s*[^0-9]+\()([0-9]+)(\s*->\s*)([0-9]+)(\)\s*regressed\s*by\s*[+-])([0-9.]+)(%\s*\([><][+-])([0-9.]+)(%\)\s*)$")
            .expect("Regex should compile");
    // Instructions (357182 -> 357704): +0.14614% exceeds limit of +0.00000%
    // $1<__NUM__>$3<__NUM__>$5<__PERCENT__>$7<__PERCENT__>$9
    static ref SUMMARY_REGRESSION_RE: Regex =
        Regex::new(r"^(\s*[^0-9]+\()([0-9]+)(\s*->\s*)([0-9]+)(\):\s*[+-])([0-9.]+)(%\s*exceeds limit of [+-])([0-9.]+)(%\s*)$")
            .expect("Regex should compile");
    // Command: target/release/deps/test_lib_bench_threads-c2a88f916ff580f9
    static ref COMMAND_RE: Regex =
        Regex::new(r"^(Command:)(\s*target/release/deps/test_(lib|bin)_bench_.+-[a-z0-9]+\s*.*)$")
            .expect("Regex should compile");
    static ref PID_RE: Regex =
        Regex::new(r"(p?pid:\s*)([0-9]+)(\s+)?").expect("Regex should compile");
    static ref DETAILS_RE: Regex =
        Regex::new(r"^Details:").expect("Regex should compile");
    static ref NOT_DETAILS_RE: Regex =
        Regex::new(r"^(?:(?:\S)|(?:[a-zA-Z]))").expect("Regex should compile");
    // `  ## pid: <__PID__> part: 1 thread: 3   |pid: <__PID__> part: 1 thread: 3`
    static ref FRAGMENT_HEADER_RE: Regex =
        Regex::new(r"^(##(?: \S+: \S+)+)(\s*)([|].*)$").expect("Regex should compile");
    static ref ABSOLUTE_PATH_RE: Regex =
        Regex::new(r"(\s+)([/][^/]*)+").expect("Regex should compile");
    // Iai-Callgrind result: Success. 2 completed without regressions; 0 regressed; 2 benchmarks finished in 0.296s
    static ref SUMMARY_LINE_RE: Regex =
        Regex::new(r"^(Iai-Callgrind result:.*finished in\s*)([0-9.]+)(s)$").expect("Regex should compile");
}

#[derive(Debug, Clone)]
struct Benchmark {
    name: String,
    dir: PathBuf,
    bench_name: String,
    config: Config,
    dest_dir: PathBuf,
    home_dir: PathBuf,
}

#[derive(Debug)]
struct BenchmarkOutput(Output);

#[derive(Debug)]
pub struct BenchmarkRunner {
    metadata: Metadata,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupConfig {
    #[serde(default, with = "benchmark_tests::serde::runs_on")]
    runs_on: Option<RunsOn>,
    #[serde(default, with = "benchmark_tests::serde::rust_version")]
    rust_version: Option<benchmark_tests::serde::rust_version::VersionComparator>,
    runs: Vec<RunConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Config {
    template: Option<PathBuf>,
    groups: Vec<GroupConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Expected {
    #[serde(default)]
    files: Vec<PathBuf>,
    #[serde(default)]
    globs: Vec<ExpectedGlob>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ExpectedConfig {
    #[serde(default)]
    files: Option<PathBuf>,
    #[serde(default)]
    stdout: Option<PathBuf>,
    #[serde(default)]
    stderr: Option<PathBuf>,
    #[serde(default)]
    exit_code: Option<i32>,
    #[serde(default)]
    zero_metrics: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExpectedGlob {
    pattern: String,
    count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExpectedRun {
    group: String,
    function: String,
    id: Option<String>,
    expected: Expected,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExpectedRuns {
    #[serde(default)]
    home_dir: Option<PathBuf>,
    data: Vec<ExpectedRun>,
}

#[derive(Debug, Clone)]
struct Metadata {
    workspace_root: PathBuf,
    target_directory: PathBuf,
    benchmarks: Vec<Benchmark>,
    benches_dir: PathBuf,
    rust_version: VersionMeta,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RunConfig {
    #[serde(default)]
    cargo_args: Vec<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    template_data: HashMap<String, minijinja::Value>,
    #[serde(default)]
    expected: Option<ExpectedConfig>,
    #[serde(default, with = "benchmark_tests::serde::runs_on")]
    runs_on: Option<RunsOn>,
    #[serde(default)]
    rmdirs: Vec<PathBuf>,
    #[serde(default, with = "benchmark_tests::serde::rust_version")]
    rust_version: Option<benchmark_tests::serde::rust_version::VersionComparator>,
    #[serde(default)]
    flaky: Option<usize>,
}

impl Benchmark {
    pub fn new(path: &Path, _package_dir: &Path, target_dir: &Path) -> Self {
        let config: Config = serde_yaml::from_reader(File::open(path).expect("File should exist"))
            .map_err(|error| format!("Failed to deserialize '{}': {error}", path.display()))
            .expect("File should be deserializable");

        let name = path.file_name().unwrap().to_string_lossy();
        let name = name.strip_suffix(".conf.yml").unwrap().to_owned();
        let (bench_name, name) = if config.template.is_some() {
            (String::from(TEMPLATE_BENCH_NAME), name)
        } else {
            (name.clone(), name.clone())
        };

        Benchmark {
            home_dir: target_dir.join("iai"),
            dest_dir: target_dir.join("iai").join(PACKAGE).join(&bench_name),
            bench_name,
            name,
            config,
            dir: path.parent().unwrap().to_path_buf(),
        }
    }

    pub fn clean_benchmark(&self) {
        if self.dest_dir.is_dir() {
            std::fs::remove_dir_all(&self.dest_dir).unwrap();
        }
        let alt_dir = self
            .home_dir
            .join(env!("IC_BUILD_TRIPLE"))
            .join(PACKAGE)
            .join(&self.bench_name);
        if alt_dir.is_dir() {
            std::fs::remove_dir_all(&alt_dir).unwrap();
        }
    }

    pub fn backup(&self) -> Option<TempDir> {
        if self.dest_dir.is_dir() {
            let dir = tempdir().expect("Creating temporary directory should succeed");
            fs_extra::copy_items(&[&self.dest_dir], dir.path(), &CopyOptions::new()).unwrap();
            Some(dir)
        } else {
            None
        }
    }

    pub fn restore(&self, temp_dir: Option<&TempDir>) {
        self.clean_benchmark();

        if let Some(temp_dir) = temp_dir {
            let from = temp_dir.path().join(self.dest_dir.file_name().unwrap());
            fs_extra::copy_items(
                &[from],
                self.dest_dir
                    .parent()
                    .expect("Parent of benchmark directory should exist"),
                &CopyOptions::new(),
            )
            .expect("Restoring backup should succeed");
        }
    }

    pub fn run_bench(
        &self,
        cargo_args: &[String],
        args: &[String],
        capture: bool,
    ) -> BenchmarkOutput {
        let stdio = if capture {
            std::env::set_var("IAI_CALLGRIND_COLOR", "never");
            Stdio::piped
        } else {
            std::env::set_var("IAI_CALLGRIND_COLOR", "auto");
            Stdio::inherit
        };

        let mut command = std::process::Command::new(env!("CARGO"));
        command.args(["bench", "--package", PACKAGE, "--bench", &self.bench_name]);
        command.args(cargo_args);
        if capture {
            command.args(["--color", "never"]);
        }
        if !args.is_empty() {
            command.arg("--");
            command.args(args);
        }
        let output = command
            .stderr(stdio())
            .stdout(stdio())
            .output()
            .expect("Launching benchmark should succeed");

        BenchmarkOutput(output)
    }

    pub fn run_template(
        &self,
        template_path: &Path,
        cargo_args: &[String],
        args: &[String],
        template_data: &HashMap<String, minijinja::Value>,
        meta: &Metadata,
        capture: bool,
    ) -> BenchmarkOutput {
        let mut template_string = String::new();
        File::open(self.dir.join(template_path))
            .expect("File should exist")
            .read_to_string(&mut template_string)
            .expect("Reading to string should succeed");

        let mut env = Environment::new();
        env.add_template(&self.bench_name, &template_string)
            .unwrap();
        let template = env.get_template(&self.bench_name).unwrap();

        let dest = File::create(meta.get_template()).unwrap();
        template.render_to_write(template_data, dest).unwrap();

        self.run_bench(cargo_args, args, capture)
    }

    pub fn run(&self, group: &GroupConfig, meta: &Metadata, schema: &ScopedSchema<'_>) {
        if !group.runs_on.as_ref().map_or(true, |(is_target, target)| {
            if *is_target {
                target == env!("IC_BUILD_TRIPLE")
            } else {
                target != env!("IC_BUILD_TRIPLE")
            }
        }) || !group.rust_version.as_ref().map_or(true, |(cmp, version)| {
            meta.compare_rust_version(*cmp, version)
        }) {
            return;
        }

        self.clean_benchmark();

        let num_runs = group.runs.len();
        for (index, run) in group
            .runs
            .iter()
            .filter(|r| {
                r.runs_on.as_ref().map_or(true, |(is_target, target)| {
                    if *is_target {
                        target == env!("IC_BUILD_TRIPLE")
                    } else {
                        target != env!("IC_BUILD_TRIPLE")
                    }
                }) && r.rust_version.as_ref().map_or(true, |(cmp, version)| {
                    meta.compare_rust_version(*cmp, version)
                })
            })
            .enumerate()
        {
            let max_tries = run.flaky.unwrap_or(0);
            let backup_dir = if max_tries > 0 { self.backup() } else { None };

            for tries in 0..=max_tries {
                print_info(format!(
                    "Running {}: ({}/{})",
                    &self.name,
                    index + 1,
                    num_runs
                ));

                for r in run.rmdirs.iter().filter(|r| r.is_dir()) {
                    print_info(format!("Removing directory: {}", r.display()));
                    std::fs::remove_dir_all(r).unwrap();
                }

                if !run.cargo_args.is_empty() {
                    print_info(format!("Cargo arguments: {}", run.cargo_args.join(" ")))
                }

                if !run.args.is_empty() {
                    print_info(format!("Benchmark arguments: {}", run.args.join(" ")))
                }

                let capture = run
                    .expected
                    .as_ref()
                    .is_some_and(|e| e.stdout.is_some() || e.stderr.is_some());

                let output = if let Some(template) = &self.config.template {
                    let output = self.run_template(
                        template,
                        &run.cargo_args,
                        &run.args,
                        &run.template_data,
                        meta,
                        capture,
                    );
                    self.reset_template(meta);
                    output
                } else {
                    self.run_bench(&run.cargo_args, &run.args, capture)
                };

                if tries < max_tries {
                    if panic::catch_unwind(AssertUnwindSafe(|| {
                        run.assert(
                            &self.dir,
                            meta,
                            output,
                            schema,
                            &self.home_dir,
                            &self.bench_name,
                        )
                    }))
                    .is_ok()
                    {
                        break;
                    } else {
                        print_info(format!(
                            "Flaky test: Re-running {}: ({}/{max_tries})",
                            &self.name,
                            tries + 1,
                        ));
                        self.restore(backup_dir.as_ref());
                    }
                } else {
                    run.assert(
                        &self.dir,
                        meta,
                        output,
                        schema,
                        &self.home_dir,
                        &self.bench_name,
                    )
                }
            }

            drop(backup_dir);
        }
    }

    fn reset_template(&self, meta: &Metadata) {
        let mut file = File::create(meta.get_template()).unwrap();
        file.write_all(TEMPLATE_CONTENT.as_bytes()).unwrap();
    }
}

impl BenchmarkOutput {
    fn assert(&self, bench_dir: &Path, _meta: &Metadata, expected: &ExpectedConfig) {
        let output = &self.0;

        print_info("STDERR:");
        stderr().write_all(&output.stderr).unwrap();
        print_info("STDOUT:");
        stdout().write_all(&output.stdout).unwrap();

        if let Some(stderr) = &expected.stderr {
            let mut expected_stderr: Vec<u8> = Vec::new();
            File::open(bench_dir.join(stderr))
                .expect("File should exist")
                .read_to_end(&mut expected_stderr)
                .expect("Reading file should succeed");

            let filtered = self.filter_stderr(&output.stderr);
            let expected_string = String::from_utf8_lossy(&expected_stderr);

            if option_env!("BENCH_OVERWRITE").map_or(false, |s| s.eq_ignore_ascii_case("yes")) {
                if filtered != expected_string {
                    print!(
                        "{}",
                        pretty_assertions::StrComparison::new(&filtered, &expected_string)
                    );

                    File::create(bench_dir.join(stderr))
                        .expect("Opening expected stdout for writing should succeed")
                        .write_all(filtered.as_bytes())
                        .expect("Writing to expected stdout should succeed");

                    print_info("Overwriting stdout successful");
                } else {
                    print_info("Skip overwrite since verifying stderr was successful");
                }
            } else {
                if filtered != expected_string {
                    panic!(
                        "Assertion of stderr failed: {}",
                        pretty_assertions::StrComparison::new(&filtered, &expected_string)
                    );
                }

                print_info("Verifying stderr successful");
            }
        }

        if let Some(stdout) = &expected.stdout {
            let mut expected_stdout: Vec<u8> = Vec::new();
            File::open(bench_dir.join(stdout))
                .expect("File should exist")
                .read_to_end(&mut expected_stdout)
                .expect("Reading file should succeed");

            let filtered = self.filter_stdout(&output.stdout);
            let expected_string = String::from_utf8_lossy(&expected_stdout);

            if option_env!("BENCH_OVERWRITE").map_or(false, |s| s.eq_ignore_ascii_case("yes")) {
                if filtered != expected_string {
                    print!(
                        "{}",
                        pretty_assertions::StrComparison::new(&filtered, &expected_string)
                    );

                    File::create(bench_dir.join(stdout))
                        .expect("Opening expected stdout for writing should succeed")
                        .write_all(filtered.as_bytes())
                        .expect("Writing to expected stdout should succeed");

                    print_info("Overwriting stdout successful");
                } else {
                    print_info("Skip overwrite since verifying stdout was successful");
                }
            } else {
                if filtered != expected_string {
                    panic!(
                        "Assertion of stdout failed: {}",
                        pretty_assertions::StrComparison::new(&filtered, &expected_string)
                    );
                }
                print_info("Verifying stdout successful");
            }
        }
    }

    fn filter_stderr(&self, stderr: &[u8]) -> String {
        let mut result = String::new();
        let mut start = false;
        for line in stderr.lines().map(Result::unwrap) {
            if !start {
                if RUNNING_RE.is_match(&line) {
                    start = true;
                }
                continue;
            }
            let line = PROCESS_DID_NOT_EXIT_SUCCESSFULLY_RE.replace(&line, "$1<__PATH__>$3");
            let line =
                REGRESSION_RE.replace(&line, "$1<__NUM__>$3<__NUM__>$5<__PERCENT__>$7<__NUM__>$9");
            writeln!(result, "{line}").unwrap();
        }
        result
    }

    fn filter_stdout(&self, stdout: &[u8]) -> String {
        let mut result = String::new();
        let mut details = false;
        for line in stdout.lines().map(Result::unwrap) {
            let (indent, line) = if line.starts_with("  ") || line.starts_with("|") {
                (&line[0..2], &line[2..])
            } else {
                (&line[0..0], line.as_str())
            };

            // The `  Details: ...` can contain platform, toolchain specific information about a
            // tool run and make the benchmark tests flaky. So, we filter the details. The
            // (multiline) details usually look like this in the original output:
            //
            // ```
            //   Command:            target/release/deps/test_lib_bench_tools-85f9071c66a70881
            //   Details:            # Thread 1
            //                       #   Total intervals: 0 (Interval Size 100000000)
            //                       #   Total instructions: 459813
            //                       #   Total reps: 499
            //                       #   Unique reps: 5
            //                       #   Total fldcw instructions: 0
            //   Command:            target/release/sort
            //   Details:            # Thread 1
            //                       #   Total intervals: 1 (Interval Size 100000000)
            //                       #   Total instructions: 104432528
            //                       #   Total reps: 457
            //                       #   Unique reps: 4
            //                       #   Total fldcw instructions: 0
            // ```
            //
            // and are transformed into: (The benchmark `Command` is also filtered. See below.)
            //
            // ```
            //   Command: <__COMMAND__>
            //   Details: <__DETAILS__>
            //   Command:            target/release/sort
            //   Details: <__DETAILS__>
            // ```
            if details {
                if NOT_DETAILS_RE.is_match(line) {
                    details = false;
                } else {
                    continue;
                }
            } else if DETAILS_RE.is_match(line) {
                writeln!(result, "{indent}Details: <__DETAILS__>").unwrap();
                details = true;
                continue;
            }

            if let Some(caps) = NUMBERS_RE.captures(line) {
                let mut string = String::new();
                let desc = caps.name("desc").unwrap().as_str();
                let comp1 = {
                    let cap = caps.name("comp1").unwrap().as_str();
                    if cap.parse::<f64>().is_ok() {
                        " ".repeat(cap.len())
                    } else {
                        cap.to_owned()
                    }
                };
                let comp2 = {
                    let cap = caps.name("comp2").unwrap().as_str();
                    if cap.parse::<f64>().is_ok() {
                        " ".repeat(cap.len())
                    } else {
                        cap.to_owned()
                    }
                };
                write!(string, "{desc}{comp1}|{comp2}").unwrap();

                // RAM Hits (and EstimatedCycles, L1, LL Hits) events are unreliable across
                // different systems/toolchains and deviate by a few counts up or down. So to keep
                // the output comparison more reliable we change this line from (for example)
                //
                //   RAM Hits:             179|209             (-14.3541%) [-1.16760x]
                //   RAM Hits:             179|179             (No Change)
                //
                // to
                //
                //   RAM Hits:             179|209             (         )
                //
                // and
                //
                //   RAM Hits:             179|N/A             (*********)
                //
                // to
                //
                //   RAM Hits:                |N/A             (*********)
                if desc.starts_with("RAM Hits")
                    || desc.starts_with("Estimated Cycles")
                    || desc.starts_with("LL Hits")
                    || desc.starts_with("L1 Hits")
                    || desc.starts_with("SysTime")
                    || desc.starts_with("SysCpuTime")
                    || desc.starts_with("Suppressed Errors")
                    || desc.starts_with("Suppressed Contexts")
                    || desc.starts_with("At t-gmax bytes")
                    || desc.starts_with("At t-gmax blocks")
                {
                    if caps.name("diff_percent").is_some() {
                        let white1 = caps.name("white1").unwrap().as_str();
                        let percent = caps.name("percent").unwrap().as_str();
                        if percent == "(*********)" {
                            write!(string, "{white1}{percent}").unwrap();
                        } else {
                            write!(string, "{white1}(         )").unwrap();
                        }
                    }
                } else {
                    if caps.name("diff_percent").is_some() {
                        let white1 = caps.name("white1").unwrap().as_str();
                        let percent = caps.name("percent").unwrap().as_str();
                        let num = &percent[1..percent.len() - 2];
                        let pos = num.find(['+', '-']);
                        if pos.is_some() && num[pos.unwrap() + 1..].parse::<f64>().is_ok() {
                            write!(
                                string,
                                "{white1}({}{}%)",
                                &num[..pos.unwrap() + 1],
                                " ".repeat(num.len() - pos.unwrap() - 1)
                            )
                            .unwrap();
                        } else {
                            write!(string, "{white1}{percent}").unwrap();
                        }
                    }
                    if caps.name("diff_factor").is_some() {
                        let white2 = caps.name("white2").unwrap().as_str();
                        let factor = caps.name("factor").unwrap().as_str();
                        let num = &factor[1..factor.len() - 2];
                        let pos = num.find(['+', '-']);
                        if pos.is_some() && num[pos.unwrap() + 1..].parse::<f64>().is_ok() {
                            write!(
                                string,
                                "{white2}[{}{}x]",
                                &num[..pos.unwrap() + 1],
                                " ".repeat(num.len() - pos.unwrap() - 1)
                            )
                            .unwrap();
                        } else {
                            write!(string, "{white2}{factor}").unwrap();
                        }
                    }
                }
                writeln!(result, "{indent}{string}").unwrap();
            } else {
                let line = if COMMAND_RE.is_match(line) {
                    // Filter the benchmark command of library benchmarks because it has a random
                    // hash in it's name
                    COMMAND_RE.replace(line, "$1 <__COMMAND__>")
                } else {
                    // Replace absolute paths
                    ABSOLUTE_PATH_RE.replace_all(line, "$1<__ABS_PATH__>$2")
                };

                // Filter the pids and parent pids
                let line = PID_RE.replace_all(&line, |caps: &Captures| {
                    format!("{}<__PID__>{}", &caps[1], caps.get(3).map_or("", |_| " "))
                });

                // Fix the spaces after replacement of pids
                let line = FRAGMENT_HEADER_RE.replace_all(&line, |caps: &Captures| {
                    let caps_1 = &caps[1];
                    let caps_3 = &caps[3];
                    if caps_1.len() < 40 {
                        format!(
                            "{caps_1}{}{caps_3}",
                            " ".repeat(
                                iai_callgrind_runner::runner::format::LEFT_WIDTH - caps_1.len()
                            )
                        )
                    } else {
                        format!("{caps_1} {caps_3}")
                    }
                });
                let line = SUMMARY_REGRESSION_RE.replace_all(
                    &line,
                    "$1<__NUM__>$3<__NUM__>$5<__PERCENT__>$7<__PERCENT__>$9",
                );
                let line = SUMMARY_LINE_RE.replace_all(&line, "$1<__SECONDS__>$3");
                writeln!(result, "{indent}{line}").unwrap();
            }
        }

        result
    }

    fn assert_exit(&self, exit_code: Option<i32>) {
        match exit_code {
            Some(expected) => match self.0.status.code() {
                Some(code) => {
                    assert_eq!(
                        expected, code,
                        "Expected benchmark to exit with code '{expected}' but exited with code \
                         '{code}'"
                    );
                    print_info(format!(
                        "Verifying exit code was successful: Process exited with '{code}'"
                    ));
                }
                None => panic!(
                    "Expected benchmark to exit with code '{expected}' but exited with signal '{}'",
                    self.0.status.signal().unwrap()
                ),
            },
            None => assert!(
                self.0.status.success(),
                "Expected benchmark to exit with success"
            ),
        }
    }
}

impl BenchmarkRunner {
    pub fn new(benches: &[String]) -> Self {
        Self {
            metadata: Metadata::new(benches),
        }
    }

    pub fn run(&self) -> Result<(), String> {
        // We need the `summary.json` files to verify that not all costs are zero. Extracting this
        // info from the summary is much easier than doing it from the output.
        std::env::set_var("IAI_CALLGRIND_SAVE_SUMMARY", "json");
        std::env::set_var(
            "IAI_CALLGRIND_RUNNER",
            self.metadata
                .target_directory
                .join("release/iai-callgrind-runner"),
        );

        let schema: serde_json::Value = serde_json::from_reader(
            File::open(
                self.metadata
                    .workspace_root
                    .join(SCHEMA_PATH)
                    .join(format!("summary.v{SCHEMA_VERSION}.schema.json")),
            )
            .unwrap(),
        )
        .unwrap();
        let mut scope = json_schema::Scope::new();
        let compiled = scope.compile_and_return(schema, false).unwrap();

        build_iai_callgrind_runner();

        for bench in &self.metadata.benchmarks {
            for group in &bench.config.groups {
                bench.run(group, &self.metadata, &compiled);
            }
        }

        Ok(())
    }
}

impl ExpectedRun {
    pub fn assert(&self, base_dir: &Path, schema: &ScopedSchema) {
        let mut env = Environment::default();
        env.add_template("function", &self.function).unwrap();
        let template = env.get_template("function").unwrap();
        let function = template.render(TEMPLATE_DATA.get().unwrap()).unwrap();

        let dir = if let Some(id) = &self.id {
            base_dir.join(&self.group).join(format!("{function}.{id}"))
        } else {
            base_dir.join(&self.group).join(&function)
        };
        print_info(format!(
            "Running assertions in directory '{}'",
            dir.display()
        ));

        assert!(
            dir.exists(),
            "Expected benchmark directory '{}' to exist",
            dir.display()
        );

        let mut real_files = glob(&format!("{}/*", dir.display()))
            .expect("Glob pattern should compile")
            .map(Result::unwrap)
            .collect::<HashSet<PathBuf>>();

        let mut summary = None;
        for file in self.expected.files.iter().map(|f| dir.join(f)) {
            if let Some(file_name) = file.file_name() {
                if file_name == "summary.json" {
                    summary = Some(file.clone());
                }
            }
            // Iai-Callgrind does not produce empty files and if so we treat it as an error
            assert!(
                real_files.remove(&file),
                "Expected file '{}' does not exist",
                file.display()
            );
            assert_ne!(
                std::fs::metadata(&file).unwrap().len(),
                0,
                "Expected file '{}' was empty",
                file.display()
            );
        }

        for ExpectedGlob { pattern, count } in self.expected.globs.iter() {
            let pattern = &dir.join(pattern).display().to_string();
            let files = glob(pattern)
                .expect("Glob pattern should compile")
                .map(Result::unwrap)
                .collect::<Vec<PathBuf>>();

            assert_eq!(
                files.len(),
                *count,
                "Expected file count for glob '{pattern}' was {} but found {} files",
                *count,
                files.len()
            );

            for file in files.into_iter() {
                if let Some(file_name) = file.file_name() {
                    if file_name == "summary.json" {
                        summary = Some(file.clone());
                    }
                }
                real_files.remove(&file);
            }
        }

        if let Some(summary) = summary {
            print_info(format!("Validating summary {}", summary.display()));
            let instance: serde_json::Value =
                serde_json::from_reader(File::open(&summary).unwrap()).unwrap();
            let result = schema.validate(&instance);
            if !result.is_valid() {
                for error in result.errors {
                    print_error(format!("{}: Validation error: {error}", summary.display()))
                }
            }
            let (_, value) = instance
                .as_object()
                .unwrap()
                .get_key_value("version")
                .unwrap();
            assert_eq!(
                value, SCHEMA_VERSION,
                "summary json schema version mismatch"
            );
        }

        assert!(
            real_files.is_empty(),
            "Expected no other files in directory '{}' but found: {:#?}",
            dir.display(),
            real_files
        );
    }
}

impl Metadata {
    pub fn new(benches: &[String]) -> Self {
        let meta = cargo_metadata::MetadataCommand::new()
            .no_deps()
            .exec()
            .unwrap();

        let package_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let benches_dir = package_dir.join("benches");
        let workspace_root = meta.workspace_root.clone().into_std_path_buf();
        let target_directory = meta.target_directory.clone().into_std_path_buf();
        let benchmarks = glob(&format!("{}/**/*.conf.yml", benches_dir.display()))
            .unwrap()
            .map(Result::unwrap)
            .filter(|path| {
                if benches.is_empty() {
                    true
                } else {
                    let name = path.file_name().unwrap().to_string_lossy();
                    benches.contains(&name.strip_suffix(".conf.yml").unwrap().to_string())
                }
            })
            .map(|path| Benchmark::new(&path, &package_dir, &target_directory))
            .collect::<Vec<Benchmark>>();
        let rust_version = get_rust_version().expect("Rust version should be present");

        Self {
            workspace_root,
            target_directory,
            benchmarks,
            benches_dir,
            rust_version,
        }
    }

    pub fn get_template(&self) -> PathBuf {
        self.benches_dir.join(format!("{TEMPLATE_BENCH_NAME}.rs"))
    }

    pub fn compare_rust_version(&self, cmp: version_compare::Cmp, version: &str) -> bool {
        if version.starts_with(|p: char| p.is_ascii_digit()) {
            version_compare::compare_to(self.rust_version.semver.to_string(), version, cmp).unwrap()
        } else {
            let channel = match version {
                "nightly" => Channel::Nightly,
                "stable" => Channel::Stable,
                "dev" => Channel::Dev,
                "beta" => Channel::Beta,
                _ => panic!("Invalid version string: {version}"),
            };
            match cmp {
                version_compare::Cmp::Eq => self.rust_version.channel == channel,
                version_compare::Cmp::Ne => self.rust_version.channel != channel,
                _ => panic!(
                    "Invalid comparator for channel: {version}. Only '=' and '!=' are allowed."
                ),
            }
        }
    }
}

impl RunConfig {
    fn assert(
        &self,
        bench_dir: &Path,
        meta: &Metadata,
        output: BenchmarkOutput,
        schema: &ScopedSchema<'_>,
        home_dir: &Path,
        bench_name: &str,
    ) {
        if let Some(expected) = &self.expected {
            if expected.stdout.is_some() || expected.stderr.is_some() {
                output.assert(bench_dir, meta, expected);
            }
            output.assert_exit(expected.exit_code);

            if let Some(files) = &expected.files {
                let expected_runs: ExpectedRuns = serde_yaml::from_reader(
                    File::open(bench_dir.join(files)).expect("File should exist"),
                )
                .map_err(|error| format!("Failed to deserialize '{}': {error}", files.display()))
                .expect("File should be deserializable");

                let dest_dir = if let Some(home_dir) = expected_runs.home_dir {
                    home_dir.join(PACKAGE).join(bench_name)
                } else {
                    home_dir.join(PACKAGE).join(bench_name)
                };

                for expected in expected_runs.data {
                    expected.assert(&dest_dir, schema);
                }
            }
        }

        if self
            .expected
            .as_ref()
            .is_some_and(|expected| !expected.zero_metrics)
        {
            let base_dir = home_dir.join(PACKAGE).join(bench_name);
            // These checks heavily depends on the creation of the `summary.json` files, but we
            // create them per default.
            for path in glob(&format!("{}/**/summary.json", base_dir.display()))
                .unwrap()
                .map(Result::unwrap)
            {
                let summary = Summary::new(&path).unwrap();
                summary.assert_costs_not_all_zero();
                print_info("Verifying costs not all zero successful");
            }
        }
    }
}

fn build_iai_callgrind_runner() {
    print_info("Building iai-callgrind-runner");
    let status = std::process::Command::new(env!("CARGO"))
        .args(["build", "--package", "iai-callgrind-runner", "--release"])
        .status()
        .unwrap();
    assert!(status.success());
}

fn print_error<T>(message: T)
where
    T: AsRef<str>,
{
    eprintln!(
        "{}: {}: {}",
        "bench".purple().bold(),
        "Error".red().bold(),
        message.as_ref()
    );
}

fn print_info<T>(message: T)
where
    T: AsRef<str>,
{
    eprintln!("{}: {}", "bench".purple().bold(), message.as_ref());
}

fn get_rust_version() -> Option<VersionMeta> {
    rustc_version::version_meta().ok()
}

fn main() {
    let benches = std::env::args().skip(1).collect::<Vec<String>>();

    let runner = BenchmarkRunner::new(&benches);

    let mut map = HashMap::new();
    map.insert(
        "target_dir_sanitized".to_owned(),
        minijinja::Value::from_serialize(
            runner
                .metadata
                .target_directory
                .display()
                .to_string()
                .replace('/', "_"),
        ),
    );

    TEMPLATE_DATA.set(map).unwrap();

    if let Err(error) = runner.run() {
        print_error(error);
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::lib_bench("Command: target/release/deps/test_lib_bench_threads-c2a88f916ff580f9")]
    #[case::bin_bench("Command: target/release/deps/test_bin_bench_threads-c2a88f916ff580f9")]
    fn test_command_re(#[case] haystack: &str) {
        assert!(COMMAND_RE.is_match(haystack));
    }

    #[rstest]
    #[case::just_root(" /", " <__ABS_PATH__>/")]
    #[case::with_single_component(" /some", " <__ABS_PATH__>/some")]
    #[case::with_two_components(" /some/final", " <__ABS_PATH__>/final")]
    #[case::with_mixed_characters(" /wi-th_/123/final", " <__ABS_PATH__>/final")]
    #[case::with_text_before(
        "some text before /wi-th_/123/final",
        "some text before <__ABS_PATH__>/final"
    )]
    #[case::with_text_after(
        " /wi-th_/123/final some text after",
        " <__ABS_PATH__>/final some text after"
    )]
    fn test_absolute_path_re(#[case] haystack: &str, #[case] replaced: &str) {
        assert_eq!(
            ABSOLUTE_PATH_RE.replace_all(haystack, "$1<__ABS_PATH__>$2"),
            replaced
        );
    }

    #[rstest]
    #[case::instructions_positive_when_0_allowed(
        "Performance has regressed: Instructions (133 -> 196) regressed by +47.3684% (>+0.00000%)",
        "Performance has regressed: Instructions (<__NUM__> -> <__NUM__>) regressed by \
         +<__PERCENT__>% (>+<__NUM__>%)"
    )]
    fn test_regression_re(#[case] haystack: &str, #[case] replaced: &str) {
        assert_eq!(
            REGRESSION_RE.replace(
                haystack,
                "$1<__NUM__>$3<__NUM__>$5<__PERCENT__>$7<__NUM__>$9"
            ),
            replaced
        );
    }
}
