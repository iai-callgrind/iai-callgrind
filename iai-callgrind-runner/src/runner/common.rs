use std::ffi::OsString;
use std::fmt::Display;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio as StdStdio};
use std::time::{Duration, Instant};

use anyhow::Result;
use log::{debug, info, log_enabled, trace, Level};
use tempfile::TempDir;

use super::args::NoCapture;
use super::format::{OutputFormatKind, SummaryFormatter};
use super::meta::Metadata;
use super::summary::BenchmarkSummary;
use crate::api::{self, Pipe};
use crate::error::Error;
use crate::util::{copy_directory, make_absolute, write_all_to_stderr};

mod defaults {
    pub const SANDBOX_FIXTURES_FOLLOW_SYMLINKS: bool = false;
    pub const SANDBOX_ENABLED: bool = false;
}

#[derive(Debug, Clone)]
pub struct Assistant {
    kind: AssistantKind,
    group_name: Option<String>,
    indices: Option<(usize, usize)>,
    pipe: Option<Pipe>,
    envs: Vec<(OsString, OsString)>,
    run_parallel: bool,
}

#[derive(Debug, Clone)]
pub enum AssistantKind {
    Setup,
    Teardown,
}

pub type Baselines = (Option<String>, Option<String>);

/// Contains benchmark summaries of (binary, library) benchmark runs and their execution time
///
/// Used to print a final summary after all benchmarks.
#[derive(Debug, Default)]
pub struct BenchmarkSummaries {
    /// The benchmark summaries
    pub summaries: Vec<BenchmarkSummary>,
    /// The execution time of all benchmarks.
    pub total_time: Option<Duration>,
}

#[derive(Debug)]
pub struct Config {
    pub package_dir: PathBuf,
    pub bench_file: PathBuf,
    pub module_path: ModulePath,
    pub bench_bin: PathBuf,
    pub meta: Metadata,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ModulePath(String);

#[derive(Debug)]
pub struct Sandbox {
    current_dir: PathBuf,
    temp_dir: Option<TempDir>,
}

impl Assistant {
    /// The setup or teardown of the `main` macro
    pub fn new_main_assistant(
        kind: AssistantKind,
        envs: Vec<(OsString, OsString)>,
        run_parallel: bool,
    ) -> Self {
        Self {
            kind,
            group_name: None,
            indices: None,
            pipe: None,
            envs,
            run_parallel,
        }
    }

    /// The setup or teardown of a `binary_benchmark_group` or `library_benchmark_group`
    pub fn new_group_assistant(
        kind: AssistantKind,
        group_name: &str,
        envs: Vec<(OsString, OsString)>,
        run_parallel: bool,
    ) -> Self {
        Self {
            kind,
            group_name: Some(group_name.to_owned()),
            indices: None,
            pipe: None,
            envs,
            run_parallel,
        }
    }

    /// The setup or teardown function of a `Bench`
    ///
    /// This is currently only used by binary benchmarks. Library benchmarks use a completely
    /// different logic for setup and teardown functions specified in a `#[bench]`, `#[benches]` and
    /// `#[library_benchmark]` and don't need to be executed via the compiled benchmark.
    pub fn new_bench_assistant(
        kind: AssistantKind,
        group_name: &str,
        indices: (usize, usize),
        pipe: Option<Pipe>,
        envs: Vec<(OsString, OsString)>,
        run_parallel: bool,
    ) -> Self {
        Self {
            kind,
            group_name: Some(group_name.to_owned()),
            indices: Some(indices),
            pipe,
            envs,
            run_parallel,
        }
    }

    /// Run the `Assistant` by calling the benchmark binary with the needed arguments
    ///
    /// We don't run the assistant if `--load-baseline` was given on the command-line!
    pub fn run(&self, config: &Config, module_path: &ModulePath) -> Result<Option<Child>> {
        if config.meta.args.load_baseline.is_some() {
            return Ok(None);
        }

        let id = self.kind.id();
        let nocapture = config.meta.args.nocapture;

        let mut command = Command::new(&config.bench_bin);
        command.envs(self.envs.iter().cloned());
        command.arg("--iai-run");

        if let Some(group_name) = &self.group_name {
            command.arg(group_name);
        }

        command.arg(&id);

        if let Some((group_index, bench_index)) = &self.indices {
            command.args([group_index.to_string(), bench_index.to_string()]);
        }

        nocapture.apply(&mut command);

        match &self.pipe {
            Some(Pipe::Stdout) => {
                command.stdout(StdStdio::piped());
            }
            Some(Pipe::Stderr) => {
                command.stderr(StdStdio::piped());
            }
            _ => {}
        }

        if self.pipe.is_some() || self.run_parallel {
            let child = command
                .spawn()
                .map_err(|error| Error::LaunchError(config.bench_bin.clone(), error.to_string()))?;
            return Ok(Some(child));
        }

        match nocapture {
            NoCapture::False => {
                let output = command
                    .output()
                    .map_err(|error| {
                        Error::LaunchError(config.bench_bin.clone(), error.to_string())
                    })
                    .and_then(|output| {
                        if output.status.success() {
                            Ok(output)
                        } else {
                            let status = output.status;
                            Err(Error::ProcessError((
                                module_path.join(&id).to_string(),
                                Some(output),
                                status,
                                None,
                            )))
                        }
                    })?;

                if log_enabled!(Level::Info) && !output.stdout.is_empty() {
                    info!("{id} function in group '{module_path}': stdout:");
                    write_all_to_stderr(&output.stdout);
                }

                if log_enabled!(Level::Info) && !output.stderr.is_empty() {
                    info!("{id} function in group '{module_path}': stderr:");
                    write_all_to_stderr(&output.stderr);
                }
            }
            NoCapture::True | NoCapture::Stderr | NoCapture::Stdout => {
                command
                    .status()
                    .map_err(|error| {
                        Error::LaunchError(config.bench_bin.clone(), error.to_string())
                    })
                    .and_then(|status| {
                        if status.success() {
                            Ok(())
                        } else {
                            Err(Error::ProcessError((
                                format!("{module_path}::{id}"),
                                None,
                                status,
                                None,
                            )))
                        }
                    })?;
            }
        }

        Ok(None)
    }
}

impl AssistantKind {
    pub fn id(&self) -> String {
        match self {
            AssistantKind::Setup => "setup",
            AssistantKind::Teardown => "teardown",
        }
        .to_owned()
    }
}

impl BenchmarkSummaries {
    /// Add a [`BenchmarkSummary`]
    pub fn add_summary(&mut self, summary: BenchmarkSummary) {
        self.summaries.push(summary);
    }

    /// Add another `BenchmarkSummary`
    ///
    /// Ignores the execution time.
    pub fn add_other(&mut self, other: Self) {
        other.summaries.into_iter().for_each(|s| {
            self.add_summary(s);
        });
    }

    /// Return true if any regressions were encountered
    pub fn is_regressed(&self) -> bool {
        self.summaries.iter().any(BenchmarkSummary::is_regressed)
    }

    /// Set the total execution from `start` to `now`
    pub fn elapsed(&mut self, start: Instant) {
        self.total_time = Some(start.elapsed());
    }

    /// Return the number of total benchmarks
    pub fn num_benchmarks(&self) -> usize {
        self.summaries.len()
    }

    /// Print the summary if not prevented by command-line arguments
    ///
    /// If `nosummary` is true or [`OutputFormatKind`] is any kind of `JSON` format the summary is
    /// not printed.
    pub fn print(&self, nosummary: bool, output_format_kind: OutputFormatKind) {
        if !nosummary {
            SummaryFormatter::new(output_format_kind).print(self);
        }
    }
}

impl ModulePath {
    pub fn new(path: &str) -> Self {
        Self(path.to_owned())
    }

    pub fn join(&self, path: &str) -> Self {
        let new = format!("{}::{path}", self.0);
        Self(new)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn last(&self) -> Option<ModulePath> {
        self.0
            .rsplit_once("::")
            .map(|(_, last)| ModulePath::new(last))
    }

    pub fn parent(&self) -> Option<ModulePath> {
        self.0
            .rsplit_once("::")
            .map(|(prefix, _)| ModulePath::new(prefix))
    }
}

impl Display for ModulePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<ModulePath> for String {
    fn from(value: ModulePath) -> Self {
        value.to_string()
    }
}

impl From<&ModulePath> for String {
    fn from(value: &ModulePath) -> Self {
        value.to_string()
    }
}

impl Sandbox {
    pub fn setup(inner: &api::Sandbox, meta: &Metadata) -> Result<Self> {
        let enabled = inner.enabled.unwrap_or(defaults::SANDBOX_ENABLED);
        let follow_symlinks = inner
            .follow_symlinks
            .unwrap_or(defaults::SANDBOX_FIXTURES_FOLLOW_SYMLINKS);
        let current_dir = std::env::current_dir().map_err(|error| {
            Error::SandboxError(format!("Failed to detect current directory: {error}"))
        })?;

        let temp_dir = if enabled {
            debug!("Creating sandbox");

            let temp_dir = tempfile::tempdir().map_err(|error| {
                Error::SandboxError(format!("Failed creating temporary directory: {error}"))
            })?;

            for fixture in &inner.fixtures {
                if fixture.is_relative() {
                    let absolute_path = make_absolute(&meta.project_root, fixture);
                    copy_directory(&absolute_path, temp_dir.path(), follow_symlinks)?;
                } else {
                    copy_directory(fixture, temp_dir.path(), follow_symlinks)?;
                }
            }

            trace!(
                "Changing current directory to sandbox directory: '{}'",
                temp_dir.path().display()
            );

            let path = temp_dir.path();
            std::env::set_current_dir(path).map_err(|error| {
                Error::SandboxError(format!(
                    "Failed setting current directory to sandbox directory: '{error}'"
                ))
            })?;
            Some(temp_dir)
        } else {
            debug!(
                "Sandbox disabled: Running benchmarks in current directory '{}'",
                current_dir.display()
            );
            None
        };

        Ok(Self {
            current_dir,
            temp_dir,
        })
    }

    pub fn reset(self) -> Result<()> {
        if let Some(temp_dir) = self.temp_dir {
            std::env::set_current_dir(&self.current_dir).map_err(|error| {
                Error::SandboxError(format!("Failed to reset current directory: {error}"))
            })?;

            if log_enabled!(Level::Debug) {
                debug!("Removing temporary workspace");
                if let Err(error) = temp_dir.close() {
                    debug!("Error trying to delete temporary workspace: {error}");
                }
            } else {
                _ = temp_dir.close();
            }
        }

        Ok(())
    }
}
