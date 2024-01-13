use std::ffi::OsString;
use std::fmt::Display;
use std::io::stderr;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use log::{debug, info, log_enabled, trace, Level};
use tempfile::TempDir;

use super::callgrind::args::Args;
use super::callgrind::flamegraph::{
    BaselineFlamegraphGenerator, Config as FlamegraphConfig, Flamegraph, FlamegraphGenerator,
    LoadBaselineFlamegraphGenerator, SaveBaselineFlamegraphGenerator,
};
use super::callgrind::model::Costs;
use super::callgrind::parser::Sentinel;
use super::callgrind::sentinel_parser::SentinelParser;
use super::callgrind::summary_parser::SummaryParser;
use super::callgrind::{CallgrindCommand, RegressionConfig};
use super::format::{tool_headline, Header, OutputFormat, VerticalFormat};
use super::meta::Metadata;
use super::summary::{
    BaselineKind, BaselineName, BenchmarkKind, BenchmarkSummary, CallgrindRegressionSummary,
    CallgrindSummary, CostsSummary, SummaryOutput,
};
use super::tool::{
    Parser, RunOptions, ToolConfigs, ToolOutputPath, ToolOutputPathKind, ValgrindTool,
};
use super::Config;
use crate::api::{self, BinaryBenchmark, BinaryBenchmarkConfig};
use crate::error::Error;
use crate::util::{copy_directory, write_all_to_stderr};

#[derive(Debug, Clone)]
struct Assistant {
    name: String,
    kind: AssistantKind,
    bench: bool,
    callgrind_args: Args,
    regression_config: Option<RegressionConfig>,
    flamegraph_config: Option<FlamegraphConfig>,
    tools: ToolConfigs,
}

#[derive(Debug, Clone)]
enum AssistantKind {
    Setup,
    Teardown,
    Before,
    After,
}

#[derive(Debug)]
struct BaselineBenchmark {
    baseline_kind: BaselineKind,
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
    command_args: Vec<OsString>,
    run_options: RunOptions,
    callgrind_args: Args,
    flamegraph_config: Option<FlamegraphConfig>,
    regression_config: Option<RegressionConfig>,
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
struct LoadBaselineBenchmark {
    loaded_baseline: BaselineName,
    baseline: BaselineName,
}

#[derive(Debug)]
struct Runner {
    groups: Groups,
    config: Config,
    benchmark: Box<dyn Benchmark>,
}

#[derive(Debug)]
struct Sandbox {
    current_dir: PathBuf,
    temp_dir: TempDir,
}

#[derive(Debug)]
struct SaveBaselineBenchmark {
    baseline: BaselineName,
}

trait Benchmark: std::fmt::Debug {
    fn output_path(
        &self,
        benchmarkable: &dyn Benchmarkable,
        config: &Config,
        group: &Group,
    ) -> ToolOutputPath;
    fn baselines(&self) -> (Option<String>, Option<String>);
    fn run(
        &self,
        benchmarkable: &dyn Benchmarkable,
        config: &Config,
        group: &Group,
    ) -> Result<BenchmarkSummary>;
}

trait Benchmarkable {
    fn callgrind_args(&self) -> &Args;
    fn executable(&self, config: &Config) -> PathBuf;
    fn executable_args(&self, config: &Config, group: &Group) -> Vec<OsString>;
    fn flamegraph_config(&self) -> Option<&FlamegraphConfig>;
    fn name(&self) -> String;
    fn run_options(&self, config: &Config) -> RunOptions;
    fn regression_config(&self) -> Option<&RegressionConfig>;
    fn tools(&self) -> &ToolConfigs;
    fn create_benchmark_summary(
        &self,
        config: &Config,
        group: &Group,
        out_path: &ToolOutputPath,
    ) -> Result<BenchmarkSummary>;
    fn check_and_print_regressions(
        &self,
        costs_summary: &CostsSummary,
    ) -> Vec<CallgrindRegressionSummary>;
    fn parse(&self, config: &Config, out_path: &ToolOutputPath) -> Result<CostsSummary>;
    fn parse_costs(&self, config: &Config, out_path: &ToolOutputPath) -> Result<Costs>;
    fn print_header(&self, meta: &Metadata, group: &Group) -> Header;
    fn sentinel(&self, config: &Config) -> Option<Sentinel>;
}

impl Assistant {
    /// Create a new [`Assistant`]
    fn new(
        name: String,
        kind: AssistantKind,
        bench: bool,
        callgrind_args: Args,
        regression_config: Option<RegressionConfig>,
        flamegraph_config: Option<FlamegraphConfig>,
        tools: ToolConfigs,
    ) -> Self {
        Self {
            name,
            kind,
            bench,
            callgrind_args,
            regression_config,
            flamegraph_config,
            tools,
        }
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

        if log_enabled!(Level::Info) && !stdout.is_empty() {
            info!("{} function '{}': stdout:", id, self.name);
            write_all_to_stderr(&stdout);
        }

        if log_enabled!(Level::Info) && !stderr.is_empty() {
            info!("{} function '{}': stderr:", id, self.name);
            write_all_to_stderr(&stderr);
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
    fn run(
        &mut self,
        benchmark: &dyn Benchmark,
        config: &Config,
        group: &Group,
    ) -> Result<Option<BenchmarkSummary>> {
        if self.bench {
            match self.kind {
                AssistantKind::Setup | AssistantKind::Teardown => self.bench = false,
                _ => {}
            }
            benchmark.run(&*self, config, group).map(Some)
        } else {
            self.run_plain(config, group).map(|()| None)
        }
    }
}

impl Benchmarkable for Assistant {
    fn callgrind_args(&self) -> &Args {
        &self.callgrind_args
    }

    fn executable(&self, config: &Config) -> PathBuf {
        config.bench_bin.clone()
    }

    fn executable_args(&self, config: &Config, group: &Group) -> Vec<OsString> {
        let run_id = if let Some(id) = &group.id {
            format!("{}::{}", id, self.kind.id())
        } else {
            self.kind.id()
        };
        vec![
            OsString::from("--iai-run"),
            OsString::from(run_id),
            OsString::from(format!("{}::{}", &config.module, &self.name)),
        ]
    }

    fn flamegraph_config(&self) -> Option<&FlamegraphConfig> {
        self.flamegraph_config.as_ref()
    }

    fn name(&self) -> String {
        format!("{}.{}", &self.name, self.kind.id())
    }

    fn run_options(&self, config: &Config) -> RunOptions {
        RunOptions {
            env_clear: false,
            entry_point: Some(format!("*{}::{}", &config.module, &self.name)),
            ..Default::default()
        }
    }

    fn regression_config(&self) -> Option<&RegressionConfig> {
        self.regression_config.as_ref()
    }

    fn tools(&self) -> &ToolConfigs {
        &self.tools
    }

    fn create_benchmark_summary(
        &self,
        config: &Config,
        group: &Group,
        out_path: &ToolOutputPath,
    ) -> Result<BenchmarkSummary> {
        let summary_output = if let Some(format) = config.meta.args.save_summary {
            let output = SummaryOutput::new(format, &out_path.dir);
            output.init()?;
            Some(output)
        } else {
            None
        };

        Ok(BenchmarkSummary::new(
            BenchmarkKind::BinaryBenchmark,
            config.meta.project_root.clone(),
            config.package_dir.clone(),
            config.bench_file.clone(),
            config.bench_bin.clone(),
            &[&group.module_path, &self.kind.id(), &self.name],
            None,
            None,
            summary_output,
        ))
    }

    fn check_and_print_regressions(
        &self,
        costs_summary: &CostsSummary,
    ) -> Vec<CallgrindRegressionSummary> {
        if let Some(regression_config) = &self.regression_config {
            regression_config.check_and_print(costs_summary)
        } else {
            vec![]
        }
    }

    fn parse(&self, config: &Config, out_path: &ToolOutputPath) -> Result<CostsSummary> {
        let new_costs = self.parse_costs(config, out_path)?;

        let old_path = out_path.to_base_path();
        #[allow(clippy::if_then_some_else_none)]
        let old_costs = if old_path.exists() {
            Some(self.parse_costs(config, &old_path)?)
        } else {
            None
        };

        Ok(CostsSummary::new(&new_costs, old_costs.as_ref()))
    }

    fn parse_costs(&self, config: &Config, out_path: &ToolOutputPath) -> Result<Costs> {
        // This unwrap is safe because `sentinel()` always returns Some
        let sentinel = self.sentinel(config).unwrap();
        SentinelParser::new(&sentinel).parse(out_path)
    }

    fn print_header(&self, meta: &Metadata, group: &Group) -> Header {
        let header = Header::from_segments(
            [&group.module_path, &self.kind.id(), &self.name],
            None,
            None,
        );

        if meta.args.output_format == OutputFormat::Default {
            header.print();
            if self.tools.has_tools_enabled() {
                println!("{}", tool_headline(ValgrindTool::Callgrind));
            }
        }

        header
    }

    fn sentinel(&self, config: &Config) -> Option<Sentinel> {
        Some(Sentinel::from_path(&config.module, &self.name))
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

impl Benchmarkable for BinBench {
    fn callgrind_args(&self) -> &Args {
        &self.callgrind_args
    }

    fn executable(&self, _config: &Config) -> PathBuf {
        self.command.clone()
    }

    fn executable_args(&self, _config: &Config, _group: &Group) -> Vec<OsString> {
        self.command_args.clone()
    }

    fn flamegraph_config(&self) -> Option<&FlamegraphConfig> {
        self.flamegraph_config.as_ref()
    }

    fn name(&self) -> String {
        format!("{}.{}", self.display, self.id)
    }

    fn run_options(&self, _config: &Config) -> RunOptions {
        self.run_options.clone()
    }

    fn regression_config(&self) -> Option<&RegressionConfig> {
        self.regression_config.as_ref()
    }

    fn tools(&self) -> &ToolConfigs {
        &self.tools
    }

    fn create_benchmark_summary(
        &self,
        config: &Config,
        group: &Group,
        out_path: &ToolOutputPath,
    ) -> Result<BenchmarkSummary> {
        let summary_output = if let Some(format) = config.meta.args.save_summary {
            let output = SummaryOutput::new(format, &out_path.dir);
            output.init()?;
            Some(output)
        } else {
            None
        };

        Ok(BenchmarkSummary::new(
            BenchmarkKind::BinaryBenchmark,
            config.meta.project_root.clone(),
            config.package_dir.clone(),
            config.bench_file.clone(),
            config.bench_bin.clone(),
            &[&group.module_path],
            Some(self.id.clone()),
            Some(self.to_string()),
            summary_output,
        ))
    }

    fn check_and_print_regressions(
        &self,
        costs_summary: &CostsSummary,
    ) -> Vec<CallgrindRegressionSummary> {
        if let Some(regression_config) = &self.regression_config {
            regression_config.check_and_print(costs_summary)
        } else {
            vec![]
        }
    }

    fn parse(&self, config: &Config, out_path: &ToolOutputPath) -> Result<CostsSummary> {
        let new_costs = self.parse_costs(config, out_path)?;

        let old_path = out_path.to_base_path();
        #[allow(clippy::if_then_some_else_none)]
        let old_costs = if old_path.exists() {
            Some(self.parse_costs(config, &old_path)?)
        } else {
            None
        };

        Ok(CostsSummary::new(&new_costs, old_costs.as_ref()))
    }

    fn parse_costs(&self, _config: &Config, out_path: &ToolOutputPath) -> Result<Costs> {
        SummaryParser.parse(out_path)
    }

    fn print_header(&self, meta: &Metadata, group: &Group) -> Header {
        let header = Header::new(&group.module_path, self.id.clone(), self.to_string());

        if meta.args.output_format == OutputFormat::Default {
            header.print();
            if self.tools.has_tools_enabled() {
                println!("{}", tool_headline(ValgrindTool::Callgrind));
            }
        }

        header
    }

    fn sentinel(&self, _config: &Config) -> Option<Sentinel> {
        self.run_options.entry_point.as_ref().map(Sentinel::new)
    }
}

impl Display for BinBench {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let args: Vec<String> = self
            .command_args
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
        benchmark: &dyn Benchmark,
        assistant: &mut Assistant,
        is_regressed: &mut bool,
        config: &Config,
    ) -> Result<()> {
        let fail_fast = assistant
            .regression_config
            .as_ref()
            .map_or(false, |r| r.fail_fast);
        if let Some(summary) = assistant.run(benchmark, config, self)? {
            summary.print_and_save(&config.meta.args.output_format)?;
            summary.check_regression(is_regressed, fail_fast)?;
        }

        Ok(())
    }

    fn run(
        &self,
        benchmark: &dyn Benchmark,
        is_regressed: &mut bool,
        config: &Config,
    ) -> Result<()> {
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
            self.run_assistant(benchmark, before, is_regressed, config)?;
        }

        for bench in &self.benches {
            if let Some(setup) = assists.setup.as_mut() {
                self.run_assistant(benchmark, setup, is_regressed, config)?;
            }

            let fail_fast = bench
                .regression_config
                .as_ref()
                .map_or(false, |r| r.fail_fast);
            let summary = benchmark.run(bench, config, self)?;
            summary.print_and_save(&config.meta.args.output_format)?;
            summary.check_regression(is_regressed, fail_fast)?;

            if let Some(teardown) = assists.teardown.as_mut() {
                self.run_assistant(benchmark, teardown, is_regressed, config)?;
            }
        }

        if let Some(after) = assists.after.as_mut() {
            self.run_assistant(benchmark, after, is_regressed, config)?;
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
            let flamegraph_config = config.flamegraph_config.map(std::convert::Into::into);
            let regression_config =
                api::update_option(&config.regression_config, &meta.regression_config)
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
                    command_args: args.args,
                    run_options: RunOptions {
                        env_clear: config.env_clear.unwrap_or(true),
                        current_dir: config.current_dir.clone(),
                        entry_point: config.entry_point.clone(),
                        exit_with: config.exit_with.clone(),
                        envs: envs.clone(),
                    },
                    callgrind_args: callgrind_args.clone(),
                    flamegraph_config: flamegraph_config.clone(),
                    regression_config: regression_config.clone(),
                    tools: tools.clone(),
                });
            }
        }
        Ok(benches)
    }

    fn parse_assists(
        assists: Vec<crate::api::Assistant>,
        callgrind_args: &Args,
        regression_config: Option<&RegressionConfig>,
        flamegraph_config: Option<&FlamegraphConfig>,
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
                        regression_config.cloned(),
                        flamegraph_config.cloned(),
                        tools.clone(),
                    ));
                }
                "after" => {
                    bench_assists.after = Some(Assistant::new(
                        assist.name,
                        AssistantKind::After,
                        assist.bench,
                        callgrind_args.clone(),
                        regression_config.cloned(),
                        flamegraph_config.cloned(),
                        tools.clone(),
                    ));
                }
                "setup" => {
                    bench_assists.setup = Some(Assistant::new(
                        assist.name,
                        AssistantKind::Setup,
                        assist.bench,
                        callgrind_args.clone(),
                        regression_config.cloned(),
                        flamegraph_config.cloned(),
                        tools.clone(),
                    ));
                }
                "teardown" => {
                    bench_assists.teardown = Some(Assistant::new(
                        assist.name,
                        AssistantKind::Teardown,
                        assist.bench,
                        callgrind_args.clone(),
                        regression_config.cloned(),
                        flamegraph_config.cloned(),
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
                    api::update_option(&group_config.regression_config, &meta.regression_config)
                        .map(std::convert::Into::into)
                        .as_ref(),
                    group_config.flamegraph_config.map(Into::into).as_ref(),
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
    fn run(&self, benchmark: &dyn Benchmark, config: &Config) -> Result<()> {
        let mut is_regressed = false;
        for group in &self.0 {
            group.run(benchmark, &mut is_regressed, config)?;
        }

        if is_regressed {
            Err(Error::RegressionError(false).into())
        } else {
            Ok(())
        }
    }
}

impl Benchmark for BaselineBenchmark {
    fn output_path(
        &self,
        benchmarkable: &dyn Benchmarkable,
        config: &Config,
        group: &Group,
    ) -> ToolOutputPath {
        ToolOutputPath::new(
            ToolOutputPathKind::Out,
            ValgrindTool::Callgrind,
            &self.baseline_kind,
            &config.meta.target_dir,
            &group.module_path,
            &benchmarkable.name(),
        )
    }

    fn baselines(&self) -> (Option<String>, Option<String>) {
        match &self.baseline_kind {
            BaselineKind::Old => (None, None),
            BaselineKind::Name(name) => (None, Some(name.to_string())),
        }
    }

    fn run(
        &self,
        benchmarkable: &dyn Benchmarkable,
        config: &Config,
        group: &Group,
    ) -> Result<BenchmarkSummary> {
        let callgrind_command = CallgrindCommand::new(&config.meta);
        let executable = benchmarkable.executable(config);
        let executable_args = benchmarkable.executable_args(config, group);
        let run_options = benchmarkable.run_options(config);
        let out_path = self.output_path(benchmarkable, config, group);
        out_path.init()?;
        out_path.shift()?;

        let old_path = out_path.to_base_path();
        let log_path = out_path.to_log_output();
        log_path.shift()?;

        for path in benchmarkable.tools().output_paths(&out_path) {
            path.shift()?;
            path.to_log_output().shift()?;
        }

        let mut benchmark_summary =
            benchmarkable.create_benchmark_summary(config, group, &out_path)?;

        let header = benchmarkable.print_header(&config.meta, group);

        let output = callgrind_command.run(
            benchmarkable.callgrind_args().clone(),
            &executable,
            &executable_args,
            run_options.clone(),
            &out_path,
        )?;

        let costs_summary = benchmarkable.parse(config, &out_path)?;
        VerticalFormat::default().print(&config.meta, self.baselines(), &costs_summary)?;

        output.dump_log(log::Level::Info);
        log_path.dump_log(log::Level::Info, &mut stderr())?;

        let regressions = benchmarkable.check_and_print_regressions(&costs_summary);

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                log_path.real_paths()?,
                out_path.real_paths()?,
            ));

        callgrind_summary.add_summary(
            &executable,
            &executable_args,
            &old_path,
            costs_summary,
            regressions,
        );

        if let Some(flamegraph_config) = benchmarkable.flamegraph_config().cloned() {
            callgrind_summary.flamegraphs = BaselineFlamegraphGenerator {
                baseline_kind: self.baseline_kind.clone(),
            }
            .create(
                &Flamegraph::new(header.to_title(), flamegraph_config),
                &out_path,
                benchmarkable.sentinel(config).as_ref(),
                &config.meta.project_root,
            )?;
        }

        benchmark_summary.tool_summaries = benchmarkable.tools().run(
            &config.meta,
            &executable,
            &executable_args,
            &run_options,
            &out_path,
            false,
        )?;

        Ok(benchmark_summary)
    }
}

impl Benchmark for LoadBaselineBenchmark {
    fn output_path(
        &self,
        benchmarkable: &dyn Benchmarkable,
        config: &Config,
        group: &Group,
    ) -> ToolOutputPath {
        ToolOutputPath::new(
            ToolOutputPathKind::Base(self.loaded_baseline.to_string()),
            ValgrindTool::Callgrind,
            &BaselineKind::Name(self.baseline.clone()),
            &config.meta.target_dir,
            &group.module_path,
            &benchmarkable.name(),
        )
    }

    fn baselines(&self) -> (Option<String>, Option<String>) {
        (
            Some(self.loaded_baseline.to_string()),
            Some(self.baseline.to_string()),
        )
    }

    fn run(
        &self,
        benchmarkable: &dyn Benchmarkable,
        config: &Config,
        group: &Group,
    ) -> Result<BenchmarkSummary> {
        let executable = benchmarkable.executable(config);
        let executable_args = benchmarkable.executable_args(config, group);
        let out_path = self.output_path(benchmarkable, config, group);
        let base_path = out_path.to_base_path();
        let log_path = out_path.to_log_output();

        let mut benchmark_summary =
            benchmarkable.create_benchmark_summary(config, group, &out_path)?;

        let header = benchmarkable.print_header(&config.meta, group);
        let costs_summary = benchmarkable.parse(config, &out_path)?;

        VerticalFormat::default().print(&config.meta, self.baselines(), &costs_summary)?;

        log_path.dump_log(log::Level::Info, &mut stderr())?;

        let regressions = benchmarkable.check_and_print_regressions(&costs_summary);

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                log_path.real_paths()?,
                out_path.real_paths()?,
            ));

        callgrind_summary.add_summary(
            &executable,
            &executable_args,
            &base_path,
            costs_summary,
            regressions,
        );

        if let Some(flamegraph_config) = benchmarkable.flamegraph_config().cloned() {
            callgrind_summary.flamegraphs = LoadBaselineFlamegraphGenerator {
                loaded_baseline: self.loaded_baseline.clone(),
                baseline: self.baseline.clone(),
            }
            .create(
                &Flamegraph::new(header.to_title(), flamegraph_config),
                &out_path,
                benchmarkable.sentinel(config).as_ref(),
                &config.meta.project_root,
            )?;
        }

        benchmark_summary.tool_summaries = benchmarkable
            .tools()
            .run_loaded_vs_base(&config.meta, &out_path)?;

        Ok(benchmark_summary)
    }
}

impl Runner {
    fn new(binary_benchmark: BinaryBenchmark, config: Config) -> Result<Self> {
        let groups = Groups::from_binary_benchmark(&config.module, binary_benchmark, &config.meta)?;

        let benchmark: Box<dyn Benchmark> =
            if let Some(baseline_name) = &config.meta.args.save_baseline {
                Box::new(SaveBaselineBenchmark {
                    baseline: baseline_name.clone(),
                })
            } else if let Some(baseline_name) = &config.meta.args.load_baseline {
                Box::new(LoadBaselineBenchmark {
                    loaded_baseline: baseline_name.clone(),
                    baseline: config
                        .meta
                        .args
                        .baseline
                        .as_ref()
                        .expect("A baseline should be present")
                        .clone(),
                })
            } else {
                Box::new(BaselineBenchmark {
                    baseline_kind: config
                        .meta
                        .args
                        .baseline
                        .as_ref()
                        .map_or(BaselineKind::Old, |name| BaselineKind::Name(name.clone())),
                })
            };

        Ok(Self {
            groups,
            config,
            benchmark,
        })
    }

    fn run(&self) -> Result<()> {
        self.groups.run(self.benchmark.as_ref(), &self.config)
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

        let current_dir = std::env::current_dir()
            .with_context(|| "Failed to detect current directory".to_owned())?;

        trace!(
            "Changing current directory to temporary directory: '{}'",
            temp_dir.path().display()
        );

        let path = temp_dir.path();
        std::env::set_current_dir(path).with_context(|| {
            format!(
                "Failed setting current directory to temporary workspace directory: '{}'",
                path.display()
            )
        })?;

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

impl Benchmark for SaveBaselineBenchmark {
    fn output_path(
        &self,
        benchmarkable: &dyn Benchmarkable,
        config: &Config,
        group: &Group,
    ) -> ToolOutputPath {
        ToolOutputPath::new(
            ToolOutputPathKind::Base(self.baseline.to_string()),
            ValgrindTool::Callgrind,
            &BaselineKind::Name(self.baseline.clone()),
            &config.meta.target_dir,
            &group.module_path,
            &benchmarkable.name(),
        )
    }

    fn baselines(&self) -> (Option<String>, Option<String>) {
        (
            Some(self.baseline.to_string()),
            Some(self.baseline.to_string()),
        )
    }

    fn run(
        &self,
        benchmarkable: &dyn Benchmarkable,
        config: &Config,
        group: &Group,
    ) -> Result<BenchmarkSummary> {
        let callgrind_command = CallgrindCommand::new(&config.meta);
        let executable = benchmarkable.executable(config);
        let executable_args = benchmarkable.executable_args(config, group);
        let run_options = benchmarkable.run_options(config);
        let out_path = self.output_path(benchmarkable, config, group);
        out_path.init()?;

        #[allow(clippy::if_then_some_else_none)]
        let old_costs = if out_path.exists() {
            let old_costs = benchmarkable.parse_costs(config, &out_path)?;
            out_path.clear()?;
            Some(old_costs)
        } else {
            None
        };

        let log_path = out_path.to_log_output();
        log_path.clear()?;

        let mut benchmark_summary =
            benchmarkable.create_benchmark_summary(config, group, &out_path)?;

        let header = benchmarkable.print_header(&config.meta, group);

        let output = callgrind_command.run(
            benchmarkable.callgrind_args().clone(),
            &executable,
            &executable_args,
            run_options.clone(),
            &out_path,
        )?;

        let new_costs = benchmarkable.parse_costs(config, &out_path)?;
        let costs_summary = CostsSummary::new(&new_costs, old_costs.as_ref());
        VerticalFormat::default().print(&config.meta, self.baselines(), &costs_summary)?;

        output.dump_log(log::Level::Info);
        log_path.dump_log(log::Level::Info, &mut stderr())?;

        let regressions = benchmarkable.check_and_print_regressions(&costs_summary);

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                log_path.real_paths()?,
                out_path.real_paths()?,
            ));

        callgrind_summary.add_summary(
            &executable,
            &executable_args,
            &out_path,
            costs_summary,
            regressions,
        );

        if let Some(flamegraph_config) = benchmarkable.flamegraph_config().cloned() {
            callgrind_summary.flamegraphs = SaveBaselineFlamegraphGenerator {
                baseline: self.baseline.clone(),
            }
            .create(
                &Flamegraph::new(header.to_title(), flamegraph_config),
                &out_path,
                benchmarkable.sentinel(config).as_ref(),
                &config.meta.project_root,
            )?;
        }

        benchmark_summary.tool_summaries = benchmarkable.tools().run(
            &config.meta,
            &executable,
            &executable_args,
            &run_options,
            &out_path,
            true,
        )?;

        Ok(benchmark_summary)
    }
}

pub fn run(binary_benchmark: BinaryBenchmark, config: Config) -> Result<()> {
    Runner::new(binary_benchmark, config)?.run()
}
