use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use colored::Colorize;
use glob::glob;
use iai_callgrind_runner::runner::summary::BenchmarkSummary;
use minijinja::Environment;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use valico::json_schema;
use valico::json_schema::schema::ScopedSchema;

const PACKAGE: &str = "benchmark-tests";
const TEMPLATE_BENCH_NAME: &str = "test_bench";
static TEMPLATE_DATA: OnceCell<HashMap<String, minijinja::Value>> = OnceCell::new();

#[derive(Debug, Clone)]
struct Benchmark {
    name: String,
    bench_name: String,
    config: Config,
    dest_dir: PathBuf,
}

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
    }

    pub fn run_bench(&self, args: &[String]) {
        let mut command = std::process::Command::new(env!("CARGO"));
        command.args(["bench", "--package", PACKAGE, "--bench", &self.bench_name]);
        if !args.is_empty() {
            command.arg("--");
            command.args(args);
        }
        let status = command
            .status()
            .expect("Launching benchmark should succeed");

        assert!(status.success(), "Expected run to be successful");
    }

    pub fn run_template(
        &self,
        template_path: &Path,
        args: &[String],
        template_data: &HashMap<String, minijinja::Value>,
        meta: &Metadata,
    ) {
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

        self.run_bench(args);
    }

    pub fn run(&self, group: &GroupConfig, meta: &Metadata, schema: &ScopedSchema<'_>) {
        self.clean_benchmark();

        let num_runs = group.runs.len();
        for (index, run) in group.runs.iter().enumerate() {
            let expected_runs = run.expected.as_ref().and_then(|expected| {
                expected.files.as_ref().map(|expected_files| {
                    let expected_runs: ExpectedRuns = serde_yaml::from_reader(
                        File::open(meta.get_file(expected_files)).expect("File should exist"),
                    )
                    .map_err(|error| {
                        format!(
                            "Failed to deserialize '{}': {error}",
                            expected_files.display()
                        )
                    })
                    .expect("File should be deserializable");
                    expected_runs
                })
            });

            print_info(format!(
                "Running {}: ({}/{})",
                &self.name,
                index + 1,
                num_runs
            ));

            if !run.args.is_empty() {
                print_info(format!("Benchmark arguments: {}", run.args.join(" ")))
            }

            if let Some(template) = &self.config.template {
                self.run_template(template, &run.args, &run.template_data, meta);
            } else {
                self.run_bench(&run.args);
            }

            if let Some(expected_runs) = expected_runs {
                for expected_run in expected_runs.data {
                    expected_run.assert(&self.dest_dir, schema);
                }
            }
        }
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
                    .join("iai-callgrind-runner/schemas/summary.v1.schema.json"),
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
        minijinja::Value::from_serializable(
            &runner
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
