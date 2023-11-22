use std::ffi::OsString;
use std::fmt::Display;
use std::io::stdout;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, Result};
use log::{debug, info, log_enabled, trace, Level};
use tempfile::TempDir;

use super::callgrind::args::Args;
use super::callgrind::flamegraph::{Config as FlamegraphConfig, Flamegraph};
use super::callgrind::parser::{Parser, Sentinel};
use super::callgrind::sentinel_parser::SentinelParser;
use super::callgrind::summary_parser::SummaryParser;
use super::callgrind::{CallgrindCommand, Regression};
use super::meta::Metadata;
use super::print::{Formatter, Header, VerticalFormat};
use super::summary::BenchmarkSummary;
use super::tool::{RunOptions, ToolConfigs};
use super::Config;
use crate::api::{self, BinaryBenchmark, BinaryBenchmarkConfig};
use crate::error::Error;
use crate::runner::print::tool_summary_header;
use crate::runner::summary::{BenchmarkKind, CallgrindSummary, CostsSummary, SummaryOutput};
use crate::runner::tool::{ToolOutputPath, ValgrindTool};
use crate::util::{copy_directory, write_all_to_stderr, write_all_to_stdout};

#[derive(Debug, Clone)]
struct Assistant {
    name: String,
    kind: AssistantKind,
    bench: bool,
    callgrind_args: Args,
    regression: Option<Regression>,
    flamegraph: Option<FlamegraphConfig>,
    tools: ToolConfigs,
}

#[derive(Debug, Clone)]
enum AssistantKind {
    Setup,
    Teardown,
    Before,
    After,
}

#[derive(Debug, Clone)]
struct BenchmarkAssistants {
    before: Option<Assistant>,
    after: Option<Assistant>,
    setup: Option<Assistant>,
    teardown: Option<Assistant>,
}

#[derive(Debug)]
struct BinBench {
    id: String,
    display: String,
    command: PathBuf,
    args: Vec<OsString>,
    options: RunOptions,
    callgrind_args: Args,
    flamegraph: Option<FlamegraphConfig>,
    regression: Option<Regression>,
    tools: ToolConfigs,
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

#[derive(Debug)]
struct Groups(Vec<Group>);

#[derive(Debug)]
struct Runner {
    groups: Groups,
    config: Config,
}

#[derive(Debug)]
struct Sandbox {
    current_dir: PathBuf,
    temp_dir: TempDir,
}

impl Assistant {
    /// Create a new [`Assistant`]
    fn new(
        name: String,
        kind: AssistantKind,
        bench: bool,
        callgrind_args: Args,
        regression: Option<Regression>,
        flamegraph: Option<FlamegraphConfig>,
        tools: ToolConfigs,
    ) -> Self {
        Self {
            name,
            kind,
            bench,
            callgrind_args,
            regression,
            flamegraph,
            tools,
        }
    }

    /// Run the assistant and benchmark this run
    #[allow(clippy::too_many_lines)]
    fn run_bench(&self, config: &Config, group: &Group) -> Result<BenchmarkSummary> {
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

        let output_path = ToolOutputPath::with_init(
            ValgrindTool::Callgrind,
            &config.meta.target_dir,
            &group.module_path,
            &format!("{}.{}", &self.name, self.kind.id()),
        );

        let log_path = output_path.to_log_output();
        log_path.init();

        let summary_output = config.meta.args.save_summary.map(|format| {
            let output = SummaryOutput::new(format, &output_path.dir);
            output.init();
            output
        });

        let mut benchmark_summary = BenchmarkSummary::new(
            BenchmarkKind::BinaryBenchmark,
            config.meta.project_root.clone(),
            config.package_dir.clone(),
            config.bench_file.clone(),
            config.bench_bin.clone(),
            &[&group.module_path, &self.kind.id(), &self.name],
            None,
            None,
            summary_output,
        );

        let header = Header::from_segments(
            [&group.module_path, &self.kind.id(), &self.name],
            None,
            None,
        );

        header.print();
        if self.tools.has_tools_enabled() {
            println!("{}", tool_summary_header(ValgrindTool::Callgrind));
        }

        let options = RunOptions {
            env_clear: false,
            entry_point: Some(format!("*{}::{}", &config.module, &self.name)),
            ..Default::default()
        };

        let output = command.run(
            self.callgrind_args.clone(),
            &config.bench_bin,
            &executable_args,
            options.clone(),
            &output_path,
        )?;

        let sentinel = Sentinel::from_path(&config.module, &self.name);
        let new_costs = SentinelParser::new(&sentinel).parse(&output_path)?;

        let old_output = output_path.to_old_output();

        #[allow(clippy::if_then_some_else_none)]
        let old_costs = if old_output.exists() {
            Some(SentinelParser::new(&sentinel).parse(&old_output)?)
        } else {
            None
        };

        let costs_summary = CostsSummary::new(&new_costs, old_costs.as_ref());
        let format = VerticalFormat::default().format(&costs_summary)?;
        print!("{format}");

        output.dump_log(log::Level::Info);
        log_path.dump_log(log::Level::Info, &mut stdout())?;

        let (regressions, fail_fast) = if let Some(regression) = &self.regression {
            (
                regression.check_and_print(&costs_summary),
                regression.fail_fast,
            )
        } else {
            (vec![], false)
        };

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                fail_fast,
                vec![log_path.to_path()],
                vec![output_path.to_path()],
            ));

        callgrind_summary.add_summary(
            &config.bench_bin,
            &executable_args,
            &old_output,
            costs_summary,
            regressions,
        );

        if let Some(flamegraph_config) = self.flamegraph.clone() {
            callgrind_summary.flamegraphs = Flamegraph::new(header.to_title(), flamegraph_config)
                .create(
                &output_path,
                Some(&sentinel),
                &config.meta.project_root,
            )?;
        }

        benchmark_summary.tool_summaries = self.tools.run(
            &config.meta,
            &config.bench_bin,
            &executable_args,
            &options,
            &output_path,
        )?;

        Ok(benchmark_summary)
    }

    /// Run the `Assistant` but don't benchmark it
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
            .map_err(|error| Error::LaunchError(config.bench_bin.clone(), error.to_string()))
            .and_then(|output| {
                if output.status.success() {
                    Ok((output.stdout, output.stderr))
                } else {
                    Err(Error::ProcessError((
                        format!("{}:{id}::{}", &config.bench_bin.display(), self.name),
                        output,
                        None,
                    )))
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

    /// Run the assistant
    ///
    /// If [`Assistant::bench`] is true then benchmark this run. This method sets `is_regressed` to
    /// true if a non-fatal regression occurred (but doesn't return an [`Error::RegressionError`])
    ///
    /// # Errors
    ///
    /// This method returns an [`anyhow::Error`] with sources:
    ///
    /// * [`Error::RegressionError`] if the regression was fatal
    fn run(&mut self, config: &Config, group: &Group) -> Result<Option<BenchmarkSummary>> {
        if self.bench {
            match self.kind {
                AssistantKind::Setup | AssistantKind::Teardown => self.bench = false,
                _ => {}
            }
            self.run_bench(config, group).map(Some)
        } else {
            self.run_plain(config, group).map(|()| None)
        }
    }
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

impl Default for BenchmarkAssistants {
    fn default() -> Self {
        Self::new()
    }
}

impl BinBench {
    /// Run the binary benchmark
    ///
    /// This method sets `is_regressed` to true if a non-fatal regression occurred (but doesn't
    /// return an `Error::RegressionError`)
    ///
    /// # Errors
    ///
    /// Returns an `anyhow::Error` with sources:
    /// `Error::RegressionError` if a fatal regression occurred.
    /// `Error::ParsingError` if a parsing error occurred.
    fn run(&self, config: &Config, group: &Group) -> Result<BenchmarkSummary> {
        let callgrind_command = CallgrindCommand::new(&config.meta);
        let output_path = ToolOutputPath::with_init(
            ValgrindTool::Callgrind,
            &config.meta.target_dir,
            &group.module_path,
            &format!("{}.{}", self.display, self.id),
        );

        let log_path = output_path.to_log_output();
        log_path.init();

        let summary_output = config.meta.args.save_summary.map(|format| {
            let output = SummaryOutput::new(format, &output_path.dir);
            output.init();
            output
        });

        let mut benchmark_summary = BenchmarkSummary::new(
            BenchmarkKind::BinaryBenchmark,
            config.meta.project_root.clone(),
            config.package_dir.clone(),
            config.bench_file.clone(),
            config.bench_bin.clone(),
            &[&group.module_path],
            Some(self.id.clone()),
            Some(self.to_string()),
            summary_output,
        );

        let header = Header::new(&group.module_path, self.id.clone(), self.to_string());
        header.print();

        if self.tools.has_tools_enabled() {
            println!("{}", tool_summary_header(ValgrindTool::Callgrind));
        }

        let output = callgrind_command.run(
            self.callgrind_args.clone(),
            &self.command,
            &self.args,
            self.options.clone(),
            &output_path,
        )?;

        let new_costs = SummaryParser.parse(&output_path)?;

        let old_output = output_path.to_old_output();
        #[allow(clippy::if_then_some_else_none)]
        let old_costs = if old_output.exists() {
            Some(SummaryParser.parse(&old_output)?)
        } else {
            None
        };

        let costs_summary = CostsSummary::new(&new_costs, old_costs.as_ref());
        let output_format = VerticalFormat::default().format(&costs_summary)?;
        print!("{output_format}");

        output.dump_log(log::Level::Info);
        log_path.dump_log(log::Level::Info, &mut stdout())?;

        let (regressions, fail_fast) = if let Some(regression) = &self.regression {
            (
                regression.check_and_print(&costs_summary),
                regression.fail_fast,
            )
        } else {
            (vec![], false)
        };

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                fail_fast,
                vec![log_path.to_path()],
                vec![output_path.to_path()],
            ));

        callgrind_summary.add_summary(
            &self.command,
            &self.args,
            &old_output,
            costs_summary,
            regressions,
        );

        let sentinel = self.options.entry_point.as_ref().map(Sentinel::new);
        if let Some(flamegraph_config) = self.flamegraph.clone() {
            callgrind_summary.flamegraphs = Flamegraph::new(header.to_title(), flamegraph_config)
                .create(
                &output_path,
                sentinel.as_ref(),
                &config.meta.project_root,
            )?;
        }

        benchmark_summary.tool_summaries = self.tools.run(
            &config.meta,
            &self.command,
            &self.args,
            &self.options,
            &output_path,
        )?;

        Ok(benchmark_summary)
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

impl Group {
    fn run_assistant(
        &self,
        assistant: &mut Assistant,
        is_regressed: &mut bool,
        config: &Config,
    ) -> Result<()> {
        if let Some(summary) = assistant.run(config, self)? {
            summary.save()?;
            summary.check_regression(is_regressed)?;
        }

        Ok(())
    }

    fn run(&self, is_regressed: &mut bool, config: &Config) -> Result<()> {
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
            self.run_assistant(before, is_regressed, config)?;
        }

        for bench in &self.benches {
            if let Some(setup) = assists.setup.as_mut() {
                self.run_assistant(setup, is_regressed, config)?;
            }

            let summary = bench.run(config, self)?;
            summary.save()?;
            summary.check_regression(is_regressed)?;

            if let Some(teardown) = assists.teardown.as_mut() {
                self.run_assistant(teardown, is_regressed, config)?;
            }
        }

        if let Some(after) = assists.after.as_mut() {
            self.run_assistant(after, is_regressed, config)?;
        }

        if let Some(sandbox) = sandbox {
            debug!("Removing sandbox");
            sandbox.reset();
        }

        Ok(())
    }
}

impl Groups {
    fn parse_runs(
        module_path: &str,
        cmd: &Option<api::Cmd>,
        runs: Vec<api::Run>,
        group_config: &BinaryBenchmarkConfig,
        meta: &Metadata,
    ) -> Result<Vec<BinBench>> {
        let mut benches = vec![];
        let mut counter: usize = 0;
        let meta_callgrind_args = meta.args.callgrind_args.clone().unwrap_or_default();

        for run in runs {
            if run.args.is_empty() {
                return Err(anyhow!(
                    "{module_path}: Found Run without an Argument. At least one argument must be \
                     specified: {run:?}",
                ));
            }
            let (orig, command) = if let Some(cmd) = run.cmd {
                (cmd.display, PathBuf::from(cmd.cmd))
            } else if let Some(command) = cmd {
                (command.display.clone(), PathBuf::from(&command.cmd))
            } else {
                return Err(anyhow!(
                    "{module_path}: Found Run without a command. A command must be specified \
                     either at group level or run level: {run:?}"
                ));
            };
            let config = group_config.clone().update_from_all([Some(&run.config)]);
            let envs = config.resolve_envs();
            let flamegraph = config.flamegraph.map(std::convert::Into::into);
            let regression = api::update_option(&config.regression, &meta.regression_config)
                .map(std::convert::Into::into);
            let callgrind_args =
                Args::from_raw_args(&[&config.raw_callgrind_args, &meta_callgrind_args])?;
            let tools = ToolConfigs(config.tools.0.into_iter().map(Into::into).collect());
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
                    options: RunOptions {
                        env_clear: config.env_clear.unwrap_or(true),
                        current_dir: config.current_dir.clone(),
                        entry_point: config.entry_point.clone(),
                        exit_with: config.exit_with.clone(),
                        envs: envs.clone(),
                    },
                    callgrind_args: callgrind_args.clone(),
                    flamegraph: flamegraph.clone(),
                    regression: regression.clone(),
                    tools: tools.clone(),
                });
            }
        }
        Ok(benches)
    }

    fn parse_assists(
        assists: Vec<crate::api::Assistant>,
        callgrind_args: &Args,
        regression: Option<&Regression>,
        flamegraph: Option<&FlamegraphConfig>,
        tools: &ToolConfigs,
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
                        regression.cloned(),
                        flamegraph.cloned(),
                        tools.clone(),
                    ));
                }
                "after" => {
                    bench_assists.after = Some(Assistant::new(
                        assist.name,
                        AssistantKind::After,
                        assist.bench,
                        callgrind_args.clone(),
                        regression.cloned(),
                        flamegraph.cloned(),
                        tools.clone(),
                    ));
                }
                "setup" => {
                    bench_assists.setup = Some(Assistant::new(
                        assist.name,
                        AssistantKind::Setup,
                        assist.bench,
                        callgrind_args.clone(),
                        regression.cloned(),
                        flamegraph.cloned(),
                        tools.clone(),
                    ));
                }
                "teardown" => {
                    bench_assists.teardown = Some(Assistant::new(
                        assist.name,
                        AssistantKind::Teardown,
                        assist.bench,
                        callgrind_args.clone(),
                        regression.cloned(),
                        flamegraph.cloned(),
                        tools.clone(),
                    ));
                }
                name => panic!("Unknown assistant function: {name}"),
            }
        }
        bench_assists
    }

    fn from_binary_benchmark(
        module: &str,
        benchmark: BinaryBenchmark,
        meta: &Metadata,
    ) -> Result<Self> {
        let global_config = benchmark.config;
        let meta_callgrind_args = meta.args.callgrind_args.clone().unwrap_or_default();

        let mut groups = vec![];
        for group in benchmark.groups {
            let module_path = if let Some(id) = group.id.as_ref() {
                format!("{module}::{id}")
            } else {
                module.to_owned()
            };
            let group_config = global_config
                .clone()
                .update_from_all([group.config.as_ref()]);
            let benches =
                Self::parse_runs(&module_path, &group.cmd, group.benches, &group_config, meta)?;
            let callgrind_args =
                Args::from_raw_args(&[&group_config.raw_callgrind_args, &meta_callgrind_args])?;
            let config = Group {
                id: group.id,
                module_path,
                fixtures: group_config.fixtures,
                sandbox: group_config.sandbox.unwrap_or(true),
                benches,
                assists: Self::parse_assists(
                    group.assists,
                    &callgrind_args,
                    api::update_option(&group_config.regression, &meta.regression_config)
                        .map(std::convert::Into::into)
                        .as_ref(),
                    group_config.flamegraph.map(Into::into).as_ref(),
                    &ToolConfigs(group_config.tools.0.into_iter().map(Into::into).collect()),
                ),
            };
            groups.push(config);
        }
        Ok(Self(groups))
    }

    /// Run all [`Group`] benchmarks
    ///
    /// # Errors
    ///
    /// Return an [`anyhow::Error`] with sources:
    ///
    /// * [`Error::RegressionError`] if a regression occurred.
    fn run(&self, config: &Config) -> Result<()> {
        let mut is_regressed = false;
        for group in &self.0 {
            group.run(&mut is_regressed, config)?;
        }

        if is_regressed {
            Err(Error::RegressionError(false).into())
        } else {
            Ok(())
        }
    }
}

impl Runner {
    fn generate(binary_benchmark: BinaryBenchmark, config: Config) -> Result<Self> {
        let groups = Groups::from_binary_benchmark(&config.module, binary_benchmark, &config.meta)?;
        Ok(Self { groups, config })
    }

    fn run(&self) -> Result<()> {
        self.groups.run(&self.config)
    }
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

pub fn run(binary_benchmark: BinaryBenchmark, config: Config) -> Result<()> {
    Runner::generate(binary_benchmark, config)?.run()
}
