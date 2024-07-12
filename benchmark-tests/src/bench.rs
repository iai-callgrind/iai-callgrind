// spell-checker:ignore rmdirs
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::fs::File;
use std::io::{stderr, stdout, BufRead, Read, Write as IOWrite};
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use colored::Colorize;
use glob::glob;
use iai_callgrind_runner::runner::summary::BenchmarkSummary;
use lazy_static::lazy_static;
use minijinja::Environment;
use once_cell::sync::OnceCell;
use regex::Regex;
use serde::{Deserialize, Serialize};
use valico::json_schema;
use valico::json_schema::schema::ScopedSchema;

const PACKAGE: &str = "benchmark-tests";
const TEMPLATE_BENCH_NAME: &str = "test_bench";
static TEMPLATE_DATA: OnceCell<HashMap<String, minijinja::Value>> = OnceCell::new();

lazy_static! {
    static ref NUMBERS_RE: Regex = Regex::new(
        r"(?x)
            (?<desc>\s+.+:\s*)(?<comp1>[0-9]+|N/A)\|(?<comp2>[0-9]+|N/A)
            (?<diff>
                (?<diff_percent>(?<white1>\s*)(?<percent>\(.*\)))
                (?<diff_factor>(?<white2>\s*)(?<factor>\[.*\]))?
            )?"
    )
    .expect("Regex should compile");
    static ref RUNNING_RE: Regex = Regex::new(r"^[ ]+Running .*$").expect("Regex should compile");
}

#[derive(Debug, Clone)]
struct Benchmark {
    name: String,
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
    summary: Option<BenchmarkSummary>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ExpectedConfig {
    #[serde(default)]
    files: Option<PathBuf>,
    #[serde(default)]
    stdout: Option<PathBuf>,
    #[serde(default)]
    stderr: Option<PathBuf>,
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct RunConfig {
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    template_data: HashMap<String, minijinja::Value>,
    expected: Option<ExpectedConfig>,
    #[serde(default)]
    runs_on: Option<String>,
    #[serde(default)]
    rmdirs: Vec<PathBuf>,
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

    pub fn run_bench(&self, args: &[String], capture: bool) -> Option<BenchmarkOutput> {
        let stdio = if capture {
            std::env::set_var("IAI_CALLGRIND_COLOR", "never");
            Stdio::piped
        } else {
            std::env::set_var("IAI_CALLGRIND_COLOR", "auto");
            Stdio::inherit
        };

        let mut command = std::process::Command::new(env!("CARGO"));
        command.args(["bench", "--package", PACKAGE, "--bench", &self.bench_name]);
        if !args.is_empty() {
            command.arg("--");
            command.args(args);
        }
        let output = command
            .stderr(stdio())
            .stdout(stdio())
            .output()
            .expect("Launching benchmark should succeed");

        assert!(output.status.success(), "Expected run to be successful");

        capture.then_some(BenchmarkOutput(output))
    }

    pub fn run_template(
        &self,
        template_path: &Path,
        args: &[String],
        template_data: &HashMap<String, minijinja::Value>,
        meta: &Metadata,
        capture: bool,
    ) -> Option<BenchmarkOutput> {
        let mut template_string = String::new();
        File::open(meta.get_file(template_path))
            .expect("File should exist")
            .read_to_string(&mut template_string)
            .expect("Reading to string should succeed");

        let mut env = Environment::new();
        env.add_template(&self.bench_name, &template_string)
            .unwrap();
        let template = env.get_template(&self.bench_name).unwrap();

        let dest = File::create(meta.get_bench_file(&self.bench_name)).unwrap();
        template.render_to_write(template_data, dest).unwrap();

        self.run_bench(args, capture)
    }

    pub fn run(&self, group: &GroupConfig, meta: &Metadata, schema: &ScopedSchema<'_>) {
        self.clean_benchmark();

        let num_runs = group.runs.len();
        for (index, run) in group
            .runs
            .iter()
            .filter(|r| {
                r.runs_on
                    .as_ref()
                    .map_or(true, |r| r == env!("IC_BUILD_TRIPLE"))
            })
            .enumerate()
        {
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

            if !run.args.is_empty() {
                print_info(format!("Benchmark arguments: {}", run.args.join(" ")))
            }

            let capture = run
                .expected
                .as_ref()
                .map_or(false, |e| e.stdout.is_some() || e.stderr.is_some());

            let output = if let Some(template) = &self.config.template {
                self.run_template(template, &run.args, &run.template_data, meta, capture)
            } else {
                self.run_bench(&run.args, capture)
            };

            run.assert(meta, output, schema, &self.home_dir, &self.bench_name);
        }
    }
}

impl BenchmarkOutput {
    fn assert(&self, meta: &Metadata, expected: &ExpectedConfig) {
        let output = &self.0;

        eprintln!("STDERR:");
        stderr().write_all(&output.stderr).unwrap();
        println!("STDOUT:");
        stdout().write_all(&output.stdout).unwrap();

        if let Some(stderr) = &expected.stderr {
            let mut expected_stderr: Vec<u8> = Vec::new();
            File::open(meta.get_file(stderr))
                .expect("File should exist")
                .read_to_end(&mut expected_stderr)
                .expect("Reading file should succeed");
            let actual = self.filter_stderr(&output.stderr);
            let expected_string = String::from_utf8_lossy(&expected_stderr);
            if actual != expected_string {
                panic!(
                    "Assertion of stderr failed: {}",
                    pretty_assertions::StrComparison::new(&actual, &expected_string)
                );
            }
        }

        if let Some(stdout) = &expected.stdout {
            let mut expected_stdout: Vec<u8> = Vec::new();
            File::open(meta.get_file(stdout))
                .expect("File should exist")
                .read_to_end(&mut expected_stdout)
                .expect("Reading file should succeed");
            let filtered = self.filter_stdout(&output.stdout);
            let expected_string = String::from_utf8_lossy(&expected_stdout);
            if filtered != expected_string {
                panic!(
                    "Assertion of stdout failed: {}",
                    pretty_assertions::StrComparison::new(&filtered, &expected_string)
                );
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
            writeln!(result, "{line}").unwrap();
        }
        result
    }

    fn filter_stdout(&self, stdout: &[u8]) -> String {
        let mut result = String::new();
        for line in stdout.lines().map(Result::unwrap) {
            if let Some(caps) = NUMBERS_RE.captures(&line) {
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

                // RAM Hits (and EstimatedCycles, L2 Hits) events are very unreliable across
                // different systems, so to keep the output comparison more reliable
                // we change this line from (for example)
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
                if desc.starts_with("  RAM Hits")
                    || desc.starts_with("  Estimated Cycles")
                    || desc.starts_with("  L2 Hits")
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
                        if num.parse::<f64>().is_ok() {
                            write!(
                                string,
                                "{white1}({}{}%)",
                                num.chars().next().unwrap(),
                                " ".repeat(num.len() - 1)
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
                        if num.parse::<f64>().is_ok() {
                            write!(
                                string,
                                "{white2}[{}{}x]",
                                num.chars().next().unwrap(),
                                " ".repeat(num.len() - 1)
                            )
                            .unwrap();
                        } else {
                            write!(string, "{white2}{factor}").unwrap();
                        }
                    }
                }
                writeln!(result, "{string}").unwrap();
            } else {
                writeln!(result, "{line}").unwrap();
            }
        }

        result
    }
}

impl BenchmarkRunner {
    #[allow(clippy::new_without_default)]
    pub fn new(benches: &[String]) -> Self {
        Self {
            metadata: Metadata::new(benches),
        }
    }

    pub fn run(&self) -> Result<(), String> {
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
                    .join("iai-callgrind-runner/schemas/summary.v2.schema.json"),
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
            assert!(
                real_files.remove(&file),
                "Expected file '{}' does not exist",
                file.display()
            );
        }

        for ExpectedGlob { pattern, count } in self.expected.globs.iter() {
            let pattern = &dir.join(pattern).display().to_string();
            let files = glob(pattern)
                .expect("Glob pattern should compile")
                .map(Result::unwrap)
                .collect::<Vec<PathBuf>>();

            assert!(
                files.len() == *count,
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
        let benchmarks = glob(&format!("{}/*.conf.yml", benches_dir.display()))
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

        Self {
            workspace_root,
            target_directory,
            benchmarks,
            benches_dir,
        }
    }

    pub fn get_bench_file(&self, bench_name: &str) -> PathBuf {
        self.get_file(format!("{bench_name}.rs"))
    }

    pub fn get_file<T>(&self, file_name: T) -> PathBuf
    where
        T: AsRef<Path>,
    {
        self.benches_dir.join(file_name.as_ref())
    }
}

impl RunConfig {
    fn assert(
        &self,
        meta: &Metadata,
        output: Option<BenchmarkOutput>,
        schema: &ScopedSchema<'_>,
        home_dir: &Path,
        bench_name: &str,
    ) {
        if let Some(expected) = &self.expected {
            if let Some(output) = output {
                output.assert(meta, expected);
            }

            if let Some(files) = &expected.files {
                let expected_runs: ExpectedRuns = serde_yaml::from_reader(
                    File::open(meta.get_file(files)).expect("File should exist"),
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
