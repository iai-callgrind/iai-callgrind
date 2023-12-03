use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::{Path, PathBuf};

use colored::Colorize;
use glob::glob;
use iai_callgrind_runner::runner::summary::BenchmarkSummary;
use new_string_template::template::Template;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use valico::json_schema;
use valico::json_schema::schema::ScopedSchema;

const PACKAGE: &str = "benchmark-tests";

#[derive(Debug, Serialize, Deserialize)]
struct ExpectedGlob {
    pattern: String,
    count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct Expected {
    #[serde(default)]
    files: Vec<PathBuf>,
    #[serde(default)]
    globs: Vec<ExpectedGlob>,
    summary: Option<BenchmarkSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExpectedRun {
    group: String,
    function: String,
    id: Option<String>,
    expected: Expected,
}

#[derive(Debug, Serialize, Deserialize)]
struct RunConfig {
    #[serde(default)]
    args: Vec<String>,
    data: Vec<ExpectedRun>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExpectedRuns {
    runs: Vec<RunConfig>,
}

#[derive(Debug, Clone)]
struct Benchmark {
    name: String,
    base_dir: PathBuf,
    expected_paths: Vec<PathBuf>,
    template_data: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct Metadata {
    workspace_root: PathBuf,
    target_directory: PathBuf,
    _package_dir: PathBuf,
    benchmarks: Vec<Benchmark>,
}

#[derive(Debug)]
pub struct BenchmarkRunner {
    metadata: Metadata,
}

impl Metadata {
    pub fn new() -> Self {
        let meta = cargo_metadata::MetadataCommand::new()
            .no_deps()
            .exec()
            .unwrap();

        let package_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = meta.workspace_root.clone().into_std_path_buf();
        let target_directory = meta.target_directory.clone().into_std_path_buf();
        let benchmarks = meta
            .workspace_packages()
            .iter()
            .find(|p| p.name == PACKAGE)
            .unwrap()
            .targets
            .iter()
            .filter(|&t| t.kind.contains(&"bench".to_owned()))
            .map(|t| Benchmark::new(&t.name, &package_dir, &target_directory))
            .collect::<Vec<Benchmark>>();

        Self {
            workspace_root,
            target_directory,
            _package_dir: package_dir,
            benchmarks,
        }
    }
}

impl Benchmark {
    pub fn new(name: &str, package_dir: &Path, target_dir: &Path) -> Self {
        let name = name.to_owned();
        let expected_paths = glob(&format!(
            "{}/{name}.expected*.json",
            package_dir.join("benches").display()
        ))
        .unwrap()
        .map(Result::unwrap)
        .collect::<Vec<PathBuf>>();
        let template_data = {
            let mut map = HashMap::new();
            map.insert(
                "target_dir_sanitized".to_owned(),
                target_dir
                    .display()
                    .to_string()
                    .replace('/', "_")
                    .to_owned(),
            );
            map
        };
        Benchmark {
            expected_paths,
            base_dir: target_dir.join("iai").join(PACKAGE).join(&name),
            name,
            template_data,
        }
    }

    pub fn is_verifiable(&self) -> bool {
        !self.expected_paths.is_empty()
    }

    pub fn clean_benchmark(&self) {
        if self.base_dir.is_dir() {
            std::fs::remove_dir_all(&self.base_dir).unwrap();
        }
    }

    pub fn run_bench(&self, args: Vec<String>) {
        let mut command = std::process::Command::new(env!("CARGO"));
        command.args(["bench", "--package", PACKAGE, "--bench", &self.name]);
        if !args.is_empty() {
            command.arg("--");
            command.args(args);
        }
        let status = command
            .status()
            .expect("Launching benchmark should succeed");

        assert!(status.success(), "Expected run to be successful");
    }

    pub fn run(&self, args: Vec<String>) {
        self.clean_benchmark();
        self.run_bench(args);
    }

    pub fn run_asserted(&self, meta: &Metadata) {
        for expected_path in &self.expected_paths {
            let expected_runs: ExpectedRuns =
                serde_json::from_reader(File::open(expected_path).expect("File should exist"))
                    .map_err(|error| {
                        format!(
                            "Failed to deserialize '{}': {error}",
                            expected_path.display()
                        )
                    })
                    .expect("File should be deserializable");

            self.clean_benchmark();

            let schema: Value = serde_json::from_reader(
                File::open(
                    meta.workspace_root
                        .join("iai-callgrind-runner/schemas/summary.v1.schema.json"),
                )
                .unwrap(),
            )
            .unwrap();
            let mut scope = json_schema::Scope::new();
            let compiled = scope.compile_and_return(schema, false).unwrap();
            for (index, runs) in expected_runs.runs.iter().enumerate() {
                print_info(format!(
                    "Running {}: ({}/{})",
                    &self.name,
                    index + 1,
                    &expected_runs.runs.len()
                ));
                if !runs.args.is_empty() {
                    print_info(format!("Benchmark arguments: {}", runs.args.join(" ")))
                }
                self.run_bench(runs.args.clone());

                for expected_run in &runs.data {
                    expected_run.assert(&self.base_dir, &self.template_data, &compiled);
                }
            }
        }
    }
}

impl BenchmarkRunner {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            metadata: Metadata::new(),
        }
    }

    pub fn run(&self, benchmarks: &[String]) -> Result<(), String> {
        std::env::set_var("IAI_CALLGRIND_SAVE_SUMMARY", "json");
        std::env::set_var(
            "IAI_CALLGRIND_RUNNER",
            self.metadata
                .target_directory
                .join("release/iai-callgrind-runner"),
        );

        let benchmarks = if benchmarks.is_empty() {
            self.metadata.benchmarks.iter().collect()
        } else {
            let mut benchmarks_to_run = vec![];
            for benchmark in benchmarks {
                if let Some(benchmark) = self
                    .metadata
                    .benchmarks
                    .iter()
                    .find(|b| &b.name == benchmark)
                {
                    benchmarks_to_run.push(benchmark);
                } else {
                    return Err(format!("Unknown benchmark: '{benchmark}'"));
                }
            }
            benchmarks_to_run
        };

        for bench in benchmarks {
            if bench.is_verifiable() {
                bench.run_asserted(&self.metadata);
            } else {
                bench.run(vec![]);
            }
        }

        Ok(())
    }
}

impl ExpectedRun {
    pub fn assert(
        &self,
        base_dir: &Path,
        template_data: &HashMap<String, String>,
        schema: &ScopedSchema,
    ) {
        let function = Template::new(&self.function)
            .render_string(template_data)
            .expect("Rendering template string should succeed");

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
            let instance: Value = serde_json::from_reader(File::open(&summary).unwrap()).unwrap();
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

    build_iai_callgrind_runner();

    let runner = BenchmarkRunner::new();

    if let Err(error) = runner.run(&benches) {
        print_error(error);
    }
}
