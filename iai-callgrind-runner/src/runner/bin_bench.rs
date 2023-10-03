use std::ffi::OsString;
use std::fmt::Display;
use std::path::PathBuf;
use std::process::Command;

use log::{debug, info, log_enabled, trace, Level};
use tempfile::TempDir;

use super::callgrind::args::CallgrindArgs;
use super::callgrind::{CallgrindCommand, CallgrindOptions, CallgrindOutput, Sentinel};
use super::meta::Metadata;
use super::print::Header;
use crate::api::{self, BinaryBenchmark, BinaryBenchmarkConfig};
use crate::error::{IaiCallgrindError, Result};
use crate::util::{copy_directory, receive_benchmark, write_all_to_stderr, write_all_to_stdout};

#[derive(Debug)]
struct BinBench {
    id: String,
    display: String,
    command: PathBuf,
    args: Vec<OsString>,
    opts: CallgrindOptions,
    callgrind_args: CallgrindArgs,
}

impl BinBench {
    fn run(&self, config: &Config, group: &Group) -> Result<()> {
        let command = CallgrindCommand::new(&config.meta);
        let output = CallgrindOutput::create(
            &config.meta.target_dir,
            &group.module_path,
            &format!("{}.{}", self.id, self.display),
        );

        command.run(
            self.callgrind_args.clone(),
            &self.command,
            &self.args,
            self.opts.clone(),
            &output,
        )?;

        let new_stats = output.parse_summary();

        let old_output = output.old_output();
        let old_stats = old_output.exists().then(|| old_output.parse_summary());

        Header::new(&group.module_path, self.id.clone(), self.to_string()).print();
        new_stats.print(old_stats);
        Ok(())
    }
}

impl Display for BinBench {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let args: Vec<String> = self
            .args
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        f.write_str(&format!(
            "{} {}",
            self.display,
            shlex::join(args.iter().map(std::string::String::as_str))
        ))
    }
}

#[derive(Debug, Clone)]
enum AssistantKind {
    Setup,
    Teardown,
    Before,
    After,
}

impl AssistantKind {
    fn id(&self) -> String {
        match self {
            AssistantKind::Setup => "setup".to_owned(),
            AssistantKind::Teardown => "teardown".to_owned(),
            AssistantKind::Before => "before".to_owned(),
            AssistantKind::After => "after".to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
struct Assistant {
    name: String,
    kind: AssistantKind,
    bench: bool,
    callgrind_args: CallgrindArgs,
}

impl Assistant {
    fn new(name: String, kind: AssistantKind, bench: bool, callgrind_args: CallgrindArgs) -> Self {
        Self {
            name,
            kind,
            bench,
            callgrind_args,
        }
    }

    fn run_bench(&self, config: &Config, group: &Group) -> Result<()> {
        let command = CallgrindCommand::new(&config.meta);

        let run_id = if let Some(id) = &group.id {
            format!("{}::{}", id, self.kind.id())
        } else {
            self.kind.id()
        };
        let executable_args = vec![
            OsString::from("--iai-run"),
            OsString::from(run_id),
            OsString::from(format!("{}::{}", &config.module, &self.name)),
        ];

        let output = CallgrindOutput::create(
            &config.meta.target_dir,
            &group.module_path,
            &format!("{}.{}", self.kind.id(), &self.name),
        );
        let options = CallgrindOptions {
            env_clear: false,
            entry_point: Some(format!("*{}::{}", &config.module, &self.name)),
            ..Default::default()
        };

        command.run(
            self.callgrind_args.clone(),
            &config.bench_bin,
            &executable_args,
            options,
            &output,
        )?;

        let sentinel = Sentinel::from_path(&config.module, &self.name);
        let new_stats = output.parse(&config.bench_file, &sentinel)?;

        let old_output = output.old_output();

        #[allow(clippy::if_then_some_else_none)]
        let old_stats = if old_output.exists() {
            Some(old_output.parse(&config.bench_file, sentinel)?)
        } else {
            None
        };

        Header::from_segments(
            [&group.module_path, &self.kind.id(), &self.name],
            None,
            None,
        )
        .print();

        new_stats.print(old_stats);
        Ok(())
    }

    fn run_plain(&self, config: &Config, group: &Group) -> Result<()> {
        let id = if let Some(id) = &group.id {
            format!("{}::{}", id, self.kind.id())
        } else {
            self.kind.id()
        };
        let mut command = Command::new(&config.bench_bin);
        command.arg("--iai-run");
        command.arg(&id);

        let (stdout, stderr) = command
            .output()
            .map_err(|error| {
                IaiCallgrindError::LaunchError(config.bench_bin.clone(), error.to_string())
            })
            .and_then(|output| {
                if output.status.success() {
                    Ok((output.stdout, output.stderr))
                } else {
                    Err(IaiCallgrindError::BenchmarkLaunchError(output))
                }
            })?;

        if !stdout.is_empty() {
            info!("{} function '{}': stdout:", id, self.name);
            if log_enabled!(Level::Info) {
                write_all_to_stdout(&stdout);
            }
        }
        if !stderr.is_empty() {
            info!("{} function '{}': stderr:", id, self.name);
            if log_enabled!(Level::Info) {
                write_all_to_stderr(&stderr);
            }
        }
        Ok(())
    }

    fn run(&mut self, config: &Config, group: &Group) -> Result<()> {
        if self.bench {
            match self.kind {
                AssistantKind::Setup | AssistantKind::Teardown => self.bench = false,
                _ => {}
            }
            self.run_bench(config, group)
        } else {
            self.run_plain(config, group)
        }
    }
}

#[derive(Debug, Clone)]
struct BenchmarkAssistants {
    before: Option<Assistant>,
    after: Option<Assistant>,
    setup: Option<Assistant>,
    teardown: Option<Assistant>,
}

impl Default for BenchmarkAssistants {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkAssistants {
    fn new() -> Self {
        Self {
            before: Option::default(),
            after: Option::default(),
            setup: Option::default(),
            teardown: Option::default(),
        }
    }
}

#[derive(Debug)]
struct Sandbox {
    current_dir: PathBuf,
    temp_dir: TempDir,
}

impl Sandbox {
    fn setup(fixtures: &Option<api::Fixtures>) -> Result<Self> {
        debug!("Creating temporary workspace directory");
        let temp_dir = tempfile::tempdir().expect("Create temporary directory");

        if let Some(fixtures) = &fixtures {
            debug!(
                "Copying fixtures from '{}' to '{}'",
                &fixtures.path.display(),
                temp_dir.path().display()
            );
            copy_directory(&fixtures.path, temp_dir.path(), fixtures.follow_symlinks)?;
        }

        let current_dir = std::env::current_dir().unwrap();
        trace!(
            "Changing current directory to temporary directory: '{}'",
            temp_dir.path().display()
        );
        std::env::set_current_dir(temp_dir.path())
            .expect("Set current directory to temporary workspace directory");

        Ok(Self {
            current_dir,
            temp_dir,
        })
    }

    fn reset(self) {
        std::env::set_current_dir(&self.current_dir)
            .expect("Reset current directory to package directory");

        if log_enabled!(Level::Debug) {
            debug!("Removing temporary workspace");
            if let Err(error) = self.temp_dir.close() {
                debug!("Error trying to delete temporary workspace: {error}");
            }
        } else {
            _ = self.temp_dir.close();
        }
    }
}

#[derive(Debug)]
struct Group {
    id: Option<String>,
    module_path: String,
    fixtures: Option<api::Fixtures>,
    sandbox: bool,
    benches: Vec<BinBench>,
    assists: BenchmarkAssistants,
}

impl Group {
    fn run(&self, config: &Config) -> Result<()> {
        let sandbox = if self.sandbox {
            debug!("Setting up sandbox");
            Some(Sandbox::setup(&self.fixtures)?)
        } else {
            debug!(
                "Sandbox switched off: Running benchmarks in the current directory: '{}'",
                std::env::current_dir().unwrap().display()
            );
            None
        };

        let mut assists = self.assists.clone();

        if let Some(before) = assists.before.as_mut() {
            before.run(config, self)?;
        }

        for bench in &self.benches {
            if let Some(setup) = assists.setup.as_mut() {
                setup.run(config, self)?;
            }

            bench.run(config, self)?;

            if let Some(teardown) = assists.teardown.as_mut() {
                teardown.run(config, self)?;
            }
        }

        if let Some(after) = assists.after.as_mut() {
            after.run(config, self)?;
        }

        if let Some(sandbox) = sandbox {
            debug!("Removing sandbox");
            sandbox.reset();
        }

        Ok(())
    }
}

#[derive(Debug)]
struct Groups(Vec<Group>);

impl Groups {
    fn parse_runs(
        module_path: &str,
        cmd: &Option<api::Cmd>,
        runs: Vec<api::Run>,
        group_config: &BinaryBenchmarkConfig,
    ) -> Result<Vec<BinBench>> {
        let mut benches = vec![];
        let mut counter: usize = 0;
        for run in runs {
            if run.args.is_empty() {
                return Err(IaiCallgrindError::Other(format!(
                    "{module_path}: Found Run without an Argument. At least one argument must be \
                     specified: {run:?}"
                )));
            }
            let (orig, command) = if let Some(cmd) = run.cmd {
                (cmd.display, PathBuf::from(cmd.cmd))
            } else if let Some(command) = cmd {
                (command.display.clone(), PathBuf::from(&command.cmd))
            } else {
                return Err(IaiCallgrindError::Other(format!(
                    "{module_path}: Found Run without a command. A command must be specified \
                     either at group level or run level: {run:?}"
                )));
            };
            let config = group_config.clone().update_from_all([Some(&run.config)]);
            let envs = config.resolve_envs();
            for args in run.args {
                let id = if let Some(id) = args.id {
                    id
                } else {
                    let id = counter.to_string();
                    counter += 1;
                    id
                };
                benches.push(BinBench {
                    id,
                    display: orig.clone(),
                    command: command.clone(),
                    args: args.args,
                    opts: CallgrindOptions {
                        env_clear: config.env_clear.unwrap_or(true),
                        current_dir: config.current_dir.clone(),
                        entry_point: config.entry_point.clone(),
                        exit_with: config.exit_with.clone(),
                        envs: envs.clone(),
                    },
                    callgrind_args: CallgrindArgs::from_raw_callgrind_args(
                        &config.raw_callgrind_args,
                    )?,
                });
            }
        }
        Ok(benches)
    }

    fn parse_assists(
        assists: Vec<crate::api::Assistant>,
        callgrind_args: &CallgrindArgs,
    ) -> BenchmarkAssistants {
        let mut bench_assists = BenchmarkAssistants::default();
        for assist in assists {
            match assist.id.as_str() {
                "before" => {
                    bench_assists.before = Some(Assistant::new(
                        assist.name,
                        AssistantKind::Before,
                        assist.bench,
                        callgrind_args.clone(),
                    ));
                }
                "after" => {
                    bench_assists.after = Some(Assistant::new(
                        assist.name,
                        AssistantKind::After,
                        assist.bench,
                        callgrind_args.clone(),
                    ));
                }
                "setup" => {
                    bench_assists.setup = Some(Assistant::new(
                        assist.name,
                        AssistantKind::Setup,
                        assist.bench,
                        callgrind_args.clone(),
                    ));
                }
                "teardown" => {
                    bench_assists.teardown = Some(Assistant::new(
                        assist.name,
                        AssistantKind::Teardown,
                        assist.bench,
                        callgrind_args.clone(),
                    ));
                }
                name => panic!("Unknown assistant function: {name}"),
            }
        }
        bench_assists
    }

    fn from_binary_benchmark(module: &str, benchmark: BinaryBenchmark) -> Result<Self> {
        // TODO: LIKE in lib_bench binary benchmarks should differentiate between command_line_args
        // and raw_callgrind_args
        let mut groups = vec![];
        for group in benchmark.groups {
            let module_path = if let Some(id) = group.id.as_ref() {
                format!("{module}::{id}")
            } else {
                module.to_owned()
            };
            let group_config = benchmark
                .config
                .clone()
                .update_from_all([group.config.as_ref()]);
            let benches = Self::parse_runs(&module_path, &group.cmd, group.benches, &group_config)?;
            let config = Group {
                id: group.id,
                module_path,
                fixtures: group_config.fixtures,
                sandbox: group_config.sandbox.unwrap_or(true),
                benches,
                assists: Self::parse_assists(
                    group.assists,
                    &CallgrindArgs::from_raw_callgrind_args(&group_config.raw_callgrind_args)?,
                ),
            };
            groups.push(config);
        }
        Ok(Self(groups))
    }

    fn run(&self, config: &Config) -> Result<()> {
        for group in &self.0 {
            group.run(config)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct Config {
    #[allow(unused)]
    package_dir: PathBuf,
    bench_file: PathBuf,
    module: String,
    bench_bin: PathBuf,
    meta: Metadata,
}

#[derive(Debug)]
struct Runner {
    groups: Groups,
    config: Config,
}

impl Runner {
    fn generate<I>(mut env_args_iter: I) -> Result<Self>
    where
        I: Iterator<Item = OsString> + std::fmt::Debug,
    {
        // The following unwraps are safe because these arguments are assuredly submitted by the
        // iai_callgrind::main macro
        let package_dir = PathBuf::from(env_args_iter.next().unwrap());
        let bench_file = PathBuf::from(env_args_iter.next().unwrap());
        let module = env_args_iter.next().unwrap().to_str().unwrap().to_owned();
        let bench_bin = PathBuf::from(env_args_iter.next().unwrap());
        let num_bytes = env_args_iter
            .next()
            .unwrap()
            .to_string_lossy()
            .parse::<usize>()
            .unwrap();

        let benchmark = receive_benchmark(num_bytes)?;
        let groups = Groups::from_binary_benchmark(&module, benchmark)?;
        let meta = Metadata::new()?;

        Ok(Self {
            config: Config {
                package_dir,
                bench_file,
                module,
                bench_bin,
                meta,
            },
            groups,
        })
    }

    fn run(&self) -> Result<()> {
        self.groups.run(&self.config)
    }
}

pub fn run<I>(env_args_iter: I) -> Result<()>
where
    I: Iterator<Item = OsString> + std::fmt::Debug,
{
    Runner::generate(env_args_iter)?.run()
}
