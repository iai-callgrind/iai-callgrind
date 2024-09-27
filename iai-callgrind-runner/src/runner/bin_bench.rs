use std::collections::HashMap;
use std::ffi::OsString;
use std::io::stderr;
use std::io::ErrorKind::WouldBlock;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Result};
use log::{debug, warn};

use super::args::NoCapture;
use super::callgrind::args::Args;
use super::callgrind::flamegraph::{
    BaselineFlamegraphGenerator, Config as FlamegraphConfig, Flamegraph, FlamegraphGenerator,
    LoadBaselineFlamegraphGenerator, SaveBaselineFlamegraphGenerator,
};
use super::callgrind::summary_parser::SummaryParser;
use super::callgrind::RegressionConfig;
use super::common::{Assistant, AssistantKind, Config, ModulePath, Sandbox};
use super::format::{BinaryBenchmarkHeader, OutputFormat, VerticalFormat};
use super::meta::Metadata;
use super::summary::{
    BaselineKind, BaselineName, BenchmarkKind, BenchmarkSummary, CallgrindSummary, CostsSummary,
    SummaryOutput,
};
use super::tool::{
    Parser, RunOptions, ToolCommand, ToolConfig, ToolConfigs, ToolOutputPath, ToolOutputPathKind,
    ValgrindTool,
};
use crate::api::{
    self, BinaryBenchmarkBench, BinaryBenchmarkConfig, BinaryBenchmarkGroups, Delay, DelayKind,
    Stdin,
};
use crate::error::Error;
use crate::runner::format;

mod defaults {
    use crate::api::Stdin;

    pub const COMPARE_BY_ID: bool = false;
    pub const ENV_CLEAR: bool = true;
    pub const REGRESSION_FAIL_FAST: bool = false;
    pub const STDIN: Stdin = Stdin::Pipe;
    pub const TRUNCATE_LENGTH: Option<usize> = Some(50);
    pub const WORKSPACE_ROOT_ENV: &str = "_WORKSPACE_ROOT";
}

#[derive(Debug)]
struct BaselineBenchmark {
    baseline_kind: BaselineKind,
}

#[derive(Debug)]
pub struct BinBench {
    pub id: Option<String>,
    pub args: Option<String>,
    pub function_name: String,
    pub command: Command,
    pub run_options: RunOptions,
    pub callgrind_args: Args,
    pub flamegraph_config: Option<FlamegraphConfig>,
    pub regression_config: Option<RegressionConfig>,
    pub tools: ToolConfigs,
    pub setup: Option<Assistant>,
    pub teardown: Option<Assistant>,
    pub sandbox: Option<api::Sandbox>,
    pub module_path: ModulePath,
    pub truncate_description: Option<usize>,
}

/// The Command we derive from the `api::Command`
///
/// If the path is relative we convert it to an absolute path relative to the workspace root.
/// `stdin`, `stdout`, `stderr` of the `api::Command` are part of the `RunOptions` and not part of
/// this `Command`
#[derive(Debug, Clone)]
pub struct Command {
    pub path: PathBuf,
    pub args: Vec<OsString>,
    pub delay: Option<Delay>,
}

#[derive(Debug)]
struct Group {
    /// This name is the name from the `library_benchmark_group!` macro
    ///
    /// Due to the way we expand the `library_benchmark_group!` macro, we can safely assume that
    /// this name is unique.
    name: String,
    /// The module path so far which should be `file_name::group_name`
    module_path: ModulePath,
    benches: Vec<BinBench>,
    setup: Option<Assistant>,
    teardown: Option<Assistant>,
    compare_by_id: bool,
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
    setup: Option<Assistant>,
    teardown: Option<Assistant>,
}

#[derive(Debug)]
struct SaveBaselineBenchmark {
    baseline: BaselineName,
}

trait Benchmark: std::fmt::Debug {
    fn output_path(&self, bin_bench: &BinBench, config: &Config, group: &Group) -> ToolOutputPath;
    fn baselines(&self) -> (Option<String>, Option<String>);
    fn run(&self, bin_bench: &BinBench, config: &Config, group: &Group)
        -> Result<BenchmarkSummary>;
}

impl Benchmark for BaselineBenchmark {
    fn output_path(&self, bin_bench: &BinBench, config: &Config, group: &Group) -> ToolOutputPath {
        ToolOutputPath::new(
            ToolOutputPathKind::Out,
            ValgrindTool::Callgrind,
            &self.baseline_kind,
            &config.meta.target_dir,
            &group.module_path,
            &bin_bench.name(),
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
        bin_bench: &BinBench,
        config: &Config,
        group: &Group,
    ) -> Result<BenchmarkSummary> {
        let header = BinaryBenchmarkHeader::new(&config.meta, bin_bench);
        header.print();

        let callgrind_command = ToolCommand::new(
            ValgrindTool::Callgrind,
            &config.meta,
            config.meta.args.nocapture,
        );

        let tool_config = ToolConfig::new(
            ValgrindTool::Callgrind,
            true,
            bin_bench.callgrind_args.clone(),
            None,
        );

        let out_path = self.output_path(bin_bench, config, group);
        out_path.init()?;
        out_path.shift()?;

        let old_path = out_path.to_base_path();
        let log_path = out_path.to_log_output();
        log_path.shift()?;

        for path in bin_bench.tools.output_paths(&out_path) {
            path.shift()?;
            path.to_log_output().shift()?;
        }

        let mut benchmark_summary = bin_bench.create_benchmark_summary(
            config,
            &out_path,
            &bin_bench.function_name,
            header.description(),
        )?;

        // We're implicitly applying the default here: In the absence of a user provided sandbox we
        // don't run the benchmarks in a sandbox. Everything from here on runs with the current
        // directory set to the sandbox directory until the sandbox is reset.
        let sandbox = bin_bench
            .sandbox
            .as_ref()
            .map(|sandbox| Sandbox::setup(sandbox, &config.meta))
            .transpose()?;

        let child = bin_bench
            .setup
            .as_ref()
            .map_or(Ok(None), |setup| setup.run(config, &bin_bench.module_path))?;

        if let Some(delay) = &bin_bench.command.delay {
            delay.clone().run()?;
        }

        let output = callgrind_command.run(
            tool_config,
            &bin_bench.command.path,
            &bin_bench.command.args,
            bin_bench.run_options.clone(),
            &out_path,
            &bin_bench.module_path,
            child,
        )?;

        if let Some(teardown) = &bin_bench.teardown {
            teardown.run(config, &bin_bench.module_path)?;
        }

        // We print the no capture footer after the teardown to keep the output consistent with
        // library benchmarks.
        bin_bench.print_nocapture_footer(config.meta.args.nocapture);

        if let Some(sandbox) = sandbox {
            sandbox.reset()?;
        }

        let new_costs = SummaryParser.parse(&out_path)?;

        let old_costs = old_path
            .exists()
            .then(|| SummaryParser.parse(&old_path))
            .transpose()?;

        let costs_summary = CostsSummary::new(&new_costs, old_costs.as_ref());
        VerticalFormat::default().print(&config.meta, self.baselines(), &costs_summary)?;

        output.dump_log(log::Level::Info);
        log_path.dump_log(log::Level::Info, &mut stderr())?;

        let regressions = bin_bench.check_and_print_regressions(&costs_summary);

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                log_path.real_paths()?,
                out_path.real_paths()?,
            ));

        callgrind_summary.add_summary(
            &bin_bench.command.path,
            &bin_bench.command.args,
            &old_path,
            costs_summary,
            regressions,
        );

        if let Some(flamegraph_config) = bin_bench.flamegraph_config.clone() {
            callgrind_summary.flamegraphs = BaselineFlamegraphGenerator {
                baseline_kind: self.baseline_kind.clone(),
            }
            .create(
                &Flamegraph::new(header.to_title(), flamegraph_config),
                &out_path,
                None,
                &config.meta.project_root,
            )?;
        }

        benchmark_summary.tool_summaries = bin_bench.tools.run(
            config,
            &bin_bench.command.path,
            &bin_bench.command.args,
            &bin_bench.run_options,
            &out_path,
            false,
            &bin_bench.module_path,
            bin_bench.sandbox.as_ref(),
            bin_bench.setup.as_ref(),
            bin_bench.teardown.as_ref(),
        )?;

        Ok(benchmark_summary)
    }
}

impl BinBench {
    fn new(
        meta: &Metadata,
        group: &Group,
        config: BinaryBenchmarkConfig,
        group_index: usize,
        bench_index: usize,
        raw_args: &api::RawArgs,
        binary_benchmark_bench: BinaryBenchmarkBench,
    ) -> Result<Self> {
        let module_path = group
            .module_path
            .join(&binary_benchmark_bench.function_name);

        let api::Command {
            path,
            args,
            stdin,
            stdout,
            stderr,
            setup_parallel,
            delay,
            ..
        } = binary_benchmark_bench.command;

        let command = Command::new(&module_path, path, args, delay)?;

        let callgrind_args = Args::from_raw_args(&[&config.raw_callgrind_args, raw_args])?;

        let mut assistant_envs = config.collect_envs();
        assistant_envs.push((
            OsString::from(defaults::WORKSPACE_ROOT_ENV),
            meta.project_root.clone().into(),
        ));

        let command_envs = config.resolve_envs();
        let flamegraph_config = config.flamegraph_config.map(Into::into);

        Ok(Self {
            id: binary_benchmark_bench.id,
            args: binary_benchmark_bench.args,
            function_name: binary_benchmark_bench.function_name,
            callgrind_args,
            flamegraph_config,
            regression_config: api::update_option(
                &config.regression_config,
                &meta.regression_config,
            )
            .map(Into::into),
            tools: ToolConfigs(config.tools.0.into_iter().map(Into::into).collect()),
            setup: binary_benchmark_bench
                .has_setup
                .then_some(Assistant::new_bench_assistant(
                    AssistantKind::Setup,
                    &group.name,
                    (group_index, bench_index),
                    stdin.as_ref().and_then(|s| {
                        if let Stdin::Setup(p) = s {
                            Some(p.clone())
                        } else {
                            None
                        }
                    }),
                    assistant_envs.clone(),
                    setup_parallel,
                )),
            teardown: binary_benchmark_bench.has_teardown.then_some(
                Assistant::new_bench_assistant(
                    AssistantKind::Teardown,
                    &group.name,
                    (group_index, bench_index),
                    None,
                    assistant_envs,
                    false,
                ),
            ),
            run_options: RunOptions {
                env_clear: config.env_clear.unwrap_or(defaults::ENV_CLEAR),
                envs: command_envs,
                stdin: stdin.or(Some(defaults::STDIN)),
                stdout,
                stderr,
                exit_with: config.exit_with,
                current_dir: config.current_dir,
            },
            sandbox: config.sandbox,
            module_path,
            command,
            truncate_description: config
                .truncate_description
                .unwrap_or(defaults::TRUNCATE_LENGTH),
        })
    }

    fn name(&self) -> String {
        if let Some(bench_id) = &self.id {
            format!("{}.{}", self.function_name, bench_id)
        } else {
            self.function_name.clone()
        }
    }

    fn print_nocapture_footer(&self, nocapture: NoCapture) {
        format::print_no_capture_footer(
            nocapture,
            self.run_options.stdout.as_ref(),
            self.run_options.stderr.as_ref(),
        );
    }

    fn create_benchmark_summary(
        &self,
        config: &Config,
        output_path: &ToolOutputPath,
        function_name: &str,
        description: Option<String>,
    ) -> Result<BenchmarkSummary> {
        let summary_output = if let Some(format) = config.meta.args.save_summary {
            let output = SummaryOutput::new(format, &output_path.dir);
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
            self.command.path.clone(),
            &self.module_path,
            function_name,
            self.id.clone(),
            description,
            summary_output,
        ))
    }

    fn check_and_print_regressions(
        &self,
        costs_summary: &CostsSummary,
    ) -> Vec<super::summary::CallgrindRegressionSummary> {
        if let Some(regression_config) = &self.regression_config {
            regression_config.check_and_print(costs_summary)
        } else {
            vec![]
        }
    }
}

impl Command {
    fn new(
        module_path: &ModulePath,
        path: PathBuf,
        args: Vec<OsString>,
        delay: Option<Delay>,
    ) -> Result<Self> {
        if path.as_os_str().is_empty() {
            return Err(anyhow!("{module_path}: Empty path in command",));
        }

        Ok(Self { path, args, delay })
    }
}

impl Group {
    fn run(
        &self,
        benchmark: &dyn Benchmark,
        is_regressed: &mut bool,
        config: &Config,
    ) -> Result<()> {
        let mut summaries: HashMap<String, Vec<BenchmarkSummary>> =
            HashMap::with_capacity(self.benches.len());
        for bench in &self.benches {
            let fail_fast = bench
                .regression_config
                .as_ref()
                .map_or(defaults::REGRESSION_FAIL_FAST, |r| r.fail_fast);

            let summary = benchmark.run(bench, config, self)?;
            summary.print_and_save(&config.meta.args.output_format)?;
            summary.check_regression(is_regressed, fail_fast)?;

            if self.compare_by_id && config.meta.args.output_format == OutputFormat::Default {
                if let Some(id) = &summary.id {
                    if let Some(sums) = summaries.get_mut(id) {
                        for sum in sums.iter() {
                            sum.compare_and_print(id, &config.meta, &summary)?;
                        }
                        sums.push(summary);
                    } else {
                        summaries.insert(id.clone(), vec![summary]);
                    }
                }
            }
        }

        Ok(())
    }
}

impl Groups {
    fn from_binary_benchmark(
        module: &ModulePath,
        benchmark_groups: BinaryBenchmarkGroups,
        meta: &Metadata,
    ) -> Result<Self> {
        let global_config = benchmark_groups.config;
        let meta_callgrind_args = meta.args.callgrind_args.clone().unwrap_or_default();

        let mut groups = vec![];
        for binary_benchmark_group in benchmark_groups.groups {
            let group_module_path = module.join(&binary_benchmark_group.id);
            let group_config = global_config
                .clone()
                .update_from_all([binary_benchmark_group.config.as_ref()]);

            let setup = binary_benchmark_group
                .has_setup
                .then_some(Assistant::new_group_assistant(
                    AssistantKind::Setup,
                    &binary_benchmark_group.id,
                    group_config.collect_envs(),
                    false,
                ));
            let teardown =
                binary_benchmark_group
                    .has_teardown
                    .then_some(Assistant::new_group_assistant(
                        AssistantKind::Teardown,
                        &binary_benchmark_group.id,
                        group_config.collect_envs(),
                        false,
                    ));

            let mut group = Group {
                name: binary_benchmark_group.id,
                module_path: group_module_path,
                benches: vec![],
                setup,
                teardown,
                compare_by_id: binary_benchmark_group
                    .compare_by_id
                    .unwrap_or(defaults::COMPARE_BY_ID),
            };

            for (group_index, binary_benchmark_benches) in binary_benchmark_group
                .binary_benchmarks
                .into_iter()
                .enumerate()
            {
                for (bench_index, binary_benchmark_bench) in
                    binary_benchmark_benches.benches.into_iter().enumerate()
                {
                    let config = group_config.clone().update_from_all([
                        binary_benchmark_benches.config.as_ref(),
                        binary_benchmark_bench.config.as_ref(),
                        Some(&binary_benchmark_bench.command.config),
                    ]);

                    let bin_bench = BinBench::new(
                        meta,
                        &group,
                        config,
                        group_index,
                        bench_index,
                        &meta_callgrind_args,
                        binary_benchmark_bench,
                    )?;
                    group.benches.push(bin_bench);
                }
            }

            groups.push(group);
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
            if let Some(setup) = &group.setup {
                setup.run(config, &group.module_path)?;
            }

            group.run(benchmark, &mut is_regressed, config)?;

            if let Some(teardown) = &group.teardown {
                teardown.run(config, &group.module_path)?;
            }
        }

        if is_regressed {
            Err(Error::RegressionError(false).into())
        } else {
            Ok(())
        }
    }
}

impl Benchmark for LoadBaselineBenchmark {
    fn output_path(&self, bin_bench: &BinBench, config: &Config, group: &Group) -> ToolOutputPath {
        ToolOutputPath::new(
            ToolOutputPathKind::Base(self.loaded_baseline.to_string()),
            ValgrindTool::Callgrind,
            &BaselineKind::Name(self.baseline.clone()),
            &config.meta.target_dir,
            &group.module_path,
            &bin_bench.name(),
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
        bin_bench: &BinBench,
        config: &Config,
        group: &Group,
    ) -> Result<BenchmarkSummary> {
        let header = BinaryBenchmarkHeader::new(&config.meta, bin_bench);
        header.print();

        let out_path = self.output_path(bin_bench, config, group);
        let old_path = out_path.to_base_path();
        let log_path = out_path.to_log_output();

        let mut benchmark_summary = bin_bench.create_benchmark_summary(
            config,
            &out_path,
            &bin_bench.function_name,
            header.description(),
        )?;

        let new_costs = SummaryParser.parse(&out_path)?;
        let old_costs = Some(SummaryParser.parse(&old_path)?);
        let costs_summary = CostsSummary::new(&new_costs, old_costs.as_ref());

        VerticalFormat::default().print(&config.meta, self.baselines(), &costs_summary)?;

        let regressions = bin_bench.check_and_print_regressions(&costs_summary);

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                log_path.real_paths()?,
                out_path.real_paths()?,
            ));

        callgrind_summary.add_summary(
            &bin_bench.command.path,
            &bin_bench.command.args,
            &old_path,
            costs_summary,
            regressions,
        );

        if let Some(flamegraph_config) = bin_bench.flamegraph_config.clone() {
            callgrind_summary.flamegraphs = LoadBaselineFlamegraphGenerator {
                loaded_baseline: self.loaded_baseline.clone(),
                baseline: self.baseline.clone(),
            }
            .create(
                &Flamegraph::new(header.to_title(), flamegraph_config),
                &out_path,
                None,
                &config.meta.project_root,
            )?;
        }

        benchmark_summary.tool_summaries = bin_bench
            .tools
            .run_loaded_vs_base(&config.meta, &out_path)?;

        Ok(benchmark_summary)
    }
}

impl Runner {
    fn new(benchmark_groups: BinaryBenchmarkGroups, config: Config) -> Result<Self> {
        let setup = benchmark_groups
            .has_setup
            .then_some(Assistant::new_main_assistant(
                AssistantKind::Setup,
                benchmark_groups.config.collect_envs(),
                false,
            ));
        let teardown = benchmark_groups
            .has_teardown
            .then_some(Assistant::new_main_assistant(
                AssistantKind::Teardown,
                benchmark_groups.config.collect_envs(),
                false,
            ));

        let groups =
            Groups::from_binary_benchmark(&config.module_path, benchmark_groups, &config.meta)?;

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
            setup,
            teardown,
        })
    }

    fn run(&self) -> Result<()> {
        if let Some(setup) = &self.setup {
            setup.run(&self.config, &self.config.module_path)?;
        }

        self.groups.run(self.benchmark.as_ref(), &self.config)?;

        if let Some(teardown) = &self.teardown {
            teardown.run(&self.config, &self.config.module_path)?;
        }
        Ok(())
    }
}

impl Benchmark for SaveBaselineBenchmark {
    fn output_path(&self, bin_bench: &BinBench, config: &Config, group: &Group) -> ToolOutputPath {
        ToolOutputPath::new(
            ToolOutputPathKind::Base(self.baseline.to_string()),
            ValgrindTool::Callgrind,
            &BaselineKind::Name(self.baseline.clone()),
            &config.meta.target_dir,
            &group.module_path,
            &bin_bench.name(),
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
        bin_bench: &BinBench,
        config: &Config,
        group: &Group,
    ) -> Result<BenchmarkSummary> {
        let header = BinaryBenchmarkHeader::new(&config.meta, bin_bench);
        header.print();

        let callgrind_command = ToolCommand::new(
            ValgrindTool::Callgrind,
            &config.meta,
            config.meta.args.nocapture,
        );

        let tool_config = ToolConfig::new(
            ValgrindTool::Callgrind,
            true,
            bin_bench.callgrind_args.clone(),
            None,
        );

        let out_path = self.output_path(bin_bench, config, group);
        out_path.init()?;

        let old_costs = out_path
            .exists()
            .then(|| {
                SummaryParser
                    .parse(&out_path)
                    .and_then(|costs| out_path.clear().map(|()| costs))
            })
            .transpose()?;

        let log_path = out_path.to_log_output();
        log_path.clear()?;

        let mut benchmark_summary = bin_bench.create_benchmark_summary(
            config,
            &out_path,
            &bin_bench.function_name,
            header.description(),
        )?;

        let sandbox = bin_bench
            .sandbox
            .as_ref()
            .map(|sandbox| Sandbox::setup(sandbox, &config.meta))
            .transpose()?;

        let child = bin_bench
            .setup
            .as_ref()
            .map_or(Ok(None), |setup| setup.run(config, &bin_bench.module_path))?;

        let output = callgrind_command.run(
            tool_config,
            &bin_bench.command.path,
            &bin_bench.command.args,
            bin_bench.run_options.clone(),
            &out_path,
            &bin_bench.module_path,
            child,
        )?;

        if let Some(teardown) = &bin_bench.teardown {
            teardown.run(config, &bin_bench.module_path)?;
        }

        bin_bench.print_nocapture_footer(config.meta.args.nocapture);

        if let Some(sandbox) = sandbox {
            sandbox.reset()?;
        }

        let new_costs = SummaryParser.parse(&out_path)?;
        let costs_summary = CostsSummary::new(&new_costs, old_costs.as_ref());
        VerticalFormat::default().print(&config.meta, self.baselines(), &costs_summary)?;

        output.dump_log(log::Level::Info);
        log_path.dump_log(log::Level::Info, &mut stderr())?;

        let regressions = bin_bench.check_and_print_regressions(&costs_summary);

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                log_path.real_paths()?,
                out_path.real_paths()?,
            ));

        callgrind_summary.add_summary(
            &bin_bench.command.path,
            &bin_bench.command.args,
            &out_path,
            costs_summary,
            regressions,
        );

        if let Some(flamegraph_config) = bin_bench.flamegraph_config.clone() {
            callgrind_summary.flamegraphs = SaveBaselineFlamegraphGenerator {
                baseline: self.baseline.clone(),
            }
            .create(
                &Flamegraph::new(header.to_title(), flamegraph_config),
                &out_path,
                None,
                &config.meta.project_root,
            )?;
        }

        benchmark_summary.tool_summaries = bin_bench.tools.run(
            config,
            &bin_bench.command.path,
            &bin_bench.command.args,
            &bin_bench.run_options,
            &out_path,
            true,
            &bin_bench.module_path,
            bin_bench.sandbox.as_ref(),
            bin_bench.setup.as_ref(),
            bin_bench.teardown.as_ref(),
        )?;

        Ok(benchmark_summary)
    }
}

pub fn run(benchmark_groups: BinaryBenchmarkGroups, config: Config) -> Result<()> {
    Runner::new(benchmark_groups, config)?.run()
}

impl Delay {
    fn defaults(mut self) -> Self {
        match self.kind {
            DelayKind::DurationElapse(_) => {
                if self.poll.is_some() {
                    warn!("Ignoring poll setting. Not supported for {:?}", self.kind);
                    self.poll = None;
                }
                if self.timeout.is_some() {
                    warn!(
                        "Ignoring timeout setting. Not supported for {:?}",
                        self.kind
                    );
                    self.timeout = None;
                }
            }
            DelayKind::PathExists(_) | DelayKind::TcpConnect(_) | DelayKind::UdpResponse(_, _) => {
                if self.timeout.is_none() {
                    self.timeout = Some(Duration::from_secs(600));
                };
                if self.poll.is_none() {
                    self.poll = Some(Duration::from_millis(10));
                };
            }
        }
        if let (Some(poll), Some(timeout)) = (self.poll, self.timeout) {
            if poll >= timeout {
                warn!(
                    "Poll duration is equal or greater than the timeout duration ({:?} >= {:?}).",
                    poll, timeout
                );

                // try to subtract -5ms to have a reasonable opportunity for success (e.g. network
                // latency)
                let diff = Duration::from_millis(5);
                if diff >= timeout {
                    self.poll = Some(timeout);
                    warn!("Updated poll duration to timeout duration {:?}", timeout);
                } else {
                    let update_poll = timeout - diff;
                    self.poll = Some(update_poll);
                    warn!(
                        "Updated poll duration to timeout duration {:?} - {:?}.",
                        timeout, diff
                    );
                }
            }
        }
        self
    }

    pub fn run(self) -> Result<()> {
        let validated = self.defaults();
        let timeout = validated.timeout;

        let (tx, rx) = mpsc::channel::<std::result::Result<(), anyhow::Error>>();
        let handle = thread::spawn(move || {
            match tx.send(validated.exec_delay_fn().map_err(|err| anyhow!(err))) {
                Ok(()) => {
                    debug!("Command::Delay successfully executed.");
                    Ok(())
                }
                Err(err) => Err(anyhow!(
                    "Command::Delay MPSC channel send error. Error: {err:?}"
                )),
            }
        });

        if let Some(timeout) = timeout {
            match rx.recv_timeout(timeout) {
                Ok(_) => Ok(()),
                Err(err) => Err(anyhow!(
                    "Command::Delay timed out or MPSC channel failed. Error: {err:?}"
                )),
            }
        } else {
            match rx.recv() {
                Ok(_) => Ok(handle.join().map_err(|err| anyhow!("{err:?}"))??),
                Err(err) => Err(anyhow!(
                    "Command::Delay MPSC channel failed. Error: {err:?}"
                )),
            }
        }
    }

    fn exec_delay_fn(&self) -> Result<()> {
        match &self.kind {
            DelayKind::DurationElapse(duration) => {
                thread::sleep(*duration);
            }
            DelayKind::TcpConnect(addr) => {
                let poll = self
                    .poll
                    .ok_or_else(|| anyhow!("DelayKind::TcpConnect requires a poll interval."))?;

                while let Err(_err) = TcpStream::connect(addr) {
                    thread::sleep(poll);
                }
            }
            DelayKind::UdpResponse(remote, req) => {
                let poll = self
                    .poll
                    .ok_or_else(|| anyhow!("DelayKind::UdpResponse requires a poll interval."))?;

                let socket = match remote {
                    SocketAddr::V4(_) => {
                        UdpSocket::bind(SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0))
                            .expect("Could not bind local IPv4 UDP socket.")
                    }
                    SocketAddr::V6(_) => {
                        UdpSocket::bind(SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), 0))
                            .expect("Could not bind local IPv6 UDP socket.")
                    }
                };

                socket.set_read_timeout(self.poll)?;
                socket.set_write_timeout(self.poll)?;

                loop {
                    while let Err(_err) = socket.send_to(req.as_slice(), remote) {
                        thread::sleep(poll);
                    }

                    let mut buf = [0; 1];
                    match socket.recv(&mut buf) {
                        Ok(_size) => break,
                        Err(e) => {
                            if e.kind() != WouldBlock {
                                thread::sleep(poll);
                            }
                        }
                    }
                }
            }
            DelayKind::PathExists(path) => {
                let poll = self
                    .poll
                    .ok_or_else(|| anyhow!("DelayKind::PathExists requires a poll interval."))?;

                let wait_for_path = std::path::PathBuf::from(Path::new(path));
                while !wait_for_path.exists() {
                    thread::sleep(poll);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs::{remove_file, File};
    use std::net::{SocketAddr, TcpListener};
    use std::time::Duration;

    use rstest::{fixture, rstest};

    use super::*;

    #[test]
    fn test_defaults() {
        // duration timeouts do not support poll & timeout
        let delay = Delay {
            poll: Some(Duration::from_secs(1)),
            timeout: Some(Duration::from_secs(1)),
            kind: DelayKind::DurationElapse(Duration::from_secs(10)),
        };

        let defaults = delay.defaults();
        assert!(defaults.poll.is_none());
        assert!(defaults.timeout.is_none());

        // default values for poll and timeout - TCP connect
        let addr = "127.0.0.1:32000".parse::<SocketAddr>().unwrap();
        let delay = Delay {
            poll: None,
            timeout: None,
            kind: DelayKind::TcpConnect(addr),
        };
        let defaults = delay.defaults();
        assert_eq!(defaults.poll.unwrap(), Duration::from_millis(10));
        assert_eq!(defaults.timeout.unwrap(), Duration::from_secs(600));

        // supplied values for poll and timeout - TCP connect
        let delay = Delay {
            poll: Some(Duration::from_millis(50)),
            timeout: Some(Duration::from_millis(200)),
            kind: DelayKind::TcpConnect(addr),
        };
        let defaults = delay.defaults();
        assert_eq!(defaults.poll.unwrap(), Duration::from_millis(50));
        assert_eq!(defaults.timeout.unwrap(), Duration::from_millis(200));

        // default values for poll and timeout - UDP request
        let delay = Delay {
            poll: None,
            timeout: None,
            kind: DelayKind::UdpResponse(addr, vec![1]),
        };
        let defaults = delay.defaults();
        assert_eq!(defaults.poll.unwrap(), Duration::from_millis(10));
        assert_eq!(defaults.timeout.unwrap(), Duration::from_secs(600));

        // supplied values for poll and timeout for UDP request
        let delay = Delay {
            poll: Some(Duration::from_millis(50)),
            timeout: Some(Duration::from_millis(200)),
            kind: DelayKind::UdpResponse(addr, vec![1]),
        };
        let defaults = delay.defaults();
        assert_eq!(defaults.poll.unwrap(), Duration::from_millis(50));
        assert_eq!(defaults.timeout.unwrap(), Duration::from_millis(200));

        // poll >= timeout, set poll duration to reasonable value
        let delay = Delay {
            poll: Some(Duration::from_millis(250)),
            timeout: Some(Duration::from_millis(200)),
            kind: DelayKind::UdpResponse(addr, vec![1]),
        };
        let defaults = delay.defaults();
        assert_eq!(defaults.poll.unwrap(), Duration::from_millis(195));
        assert_eq!(defaults.timeout.unwrap(), Duration::from_millis(200));

        // poll >= timeout, set poll duration to timeout value
        let delay = Delay {
            poll: Some(Duration::from_millis(5)),
            timeout: Some(Duration::from_millis(4)),
            kind: DelayKind::UdpResponse(addr, vec![1]),
        };
        let defaults = delay.defaults();
        assert_eq!(defaults.poll.unwrap(), Duration::from_millis(4));
        assert_eq!(defaults.timeout.unwrap(), Duration::from_millis(4));
    }

    #[fixture]
    fn delay_path_dir() -> String {
        let base_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        assert!(base_dir.ends_with("iai-callgrind/iai-callgrind-runner"));

        format!("{base_dir}/../target/tests/delay_path")
    }

    #[rstest]
    fn test_delay_path() {
        let file = "file.pid";
        let dir = delay_path_dir();
        let file_path = format!("{}/{file}", delay_path_dir());

        if !Path::new(&dir).exists() {
            std::fs::create_dir_all(dir.clone()).unwrap();
        };
        if Path::new(&file_path).exists() {
            remove_file(file_path.clone()).unwrap();
        };

        let check_file_path = file_path.clone();
        let handle = thread::spawn(move || {
            let delay = Delay {
                poll: Some(Duration::from_millis(50)),
                timeout: Some(Duration::from_millis(200)),
                kind: DelayKind::PathExists(check_file_path.into()),
            };
            delay.run().unwrap();
        });

        thread::sleep(Duration::from_millis(100));
        let _file = File::create(file_path).unwrap();

        handle.join().unwrap();
    }

    #[rstest]
    fn test_delay_tcp_connect() {
        let addr = "127.0.0.1:32000".parse::<SocketAddr>().unwrap();
        let _listener = TcpListener::bind(addr).unwrap();

        let delay = Delay {
            poll: Some(Duration::from_millis(20)),
            timeout: Some(Duration::from_secs(1)),
            kind: DelayKind::TcpConnect(addr),
        };
        delay.run().unwrap();
    }

    #[test]
    fn test_delay_tcp_connect_poll() {
        let addr = "127.0.0.1:32001".parse::<SocketAddr>().unwrap();

        let check_addr = addr;
        let handle = thread::spawn(move || {
            let delay = Delay {
                poll: Some(Duration::from_millis(20)),
                timeout: Some(Duration::from_secs(1)),
                kind: DelayKind::TcpConnect(check_addr),
            };
            delay.run().unwrap();
        });

        thread::sleep(Duration::from_millis(100));
        let _listener = TcpListener::bind(addr).unwrap();

        handle.join().unwrap();
    }

    #[test]
    fn test_delay_tcp_connect_timeout() {
        let addr = "127.0.0.1:32002".parse::<SocketAddr>().unwrap();
        let delay = Delay {
            poll: Some(Duration::from_millis(20)),
            timeout: Some(Duration::from_secs(1)),
            kind: DelayKind::TcpConnect(addr),
        };

        let result = delay.run();
        assert!(result.as_ref().err().is_some());
        assert_eq!(
            result.as_ref().err().unwrap().to_string(),
            "Command::Delay timed out or MPSC channel failed. Error: Timeout"
        );
    }

    #[rstest]
    fn test_delay_udp_response() {
        let addr = "127.0.0.1:34000".parse::<SocketAddr>().unwrap();

        thread::spawn(move || {
            let server = UdpSocket::bind(addr).unwrap();
            server
                .set_read_timeout(Some(Duration::from_millis(100)))
                .unwrap();
            server
                .set_write_timeout(Some(Duration::from_millis(100)))
                .unwrap();

            loop {
                let mut buf = [0; 1];

                match server.recv_from(&mut buf) {
                    Ok((_size, from)) => {
                        server.send_to(&[2], from).unwrap();
                    }
                    Err(_e) => {}
                }
            }
        });

        let delay = Delay {
            poll: Some(Duration::from_millis(20)),
            timeout: Some(Duration::from_millis(100)),
            kind: DelayKind::UdpResponse(addr, vec![1]),
        };
        delay.run().unwrap();
    }

    #[test]
    fn test_delay_udp_response_poll() {
        let addr = "127.0.0.1:34001".parse::<SocketAddr>().unwrap();

        thread::spawn(move || {
            let delay = Delay {
                poll: Some(Duration::from_millis(20)),
                timeout: Some(Duration::from_millis(100)),
                kind: DelayKind::UdpResponse(addr, vec![1]),
            };
            delay.run().unwrap();
        });

        let server = UdpSocket::bind(addr).unwrap();
        server
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        server
            .set_write_timeout(Some(Duration::from_millis(100)))
            .unwrap();

        loop {
            let mut buf = [0; 1];

            thread::sleep(Duration::from_millis(70));

            match server.recv_from(&mut buf) {
                Ok((_size, from)) => {
                    server.send_to(&[2], from).unwrap();
                    break;
                }
                Err(_e) => {}
            }
        }
    }

    #[test]
    fn test_delay_udp_response_timeout() {
        let addr = "127.0.0.1:34002".parse::<SocketAddr>().unwrap();
        let delay = Delay {
            poll: Some(Duration::from_millis(20)),
            timeout: Some(Duration::from_millis(100)),
            kind: DelayKind::UdpResponse(addr, vec![1]),
        };
        let result = delay.run();
        assert!(result.as_ref().err().is_some());
        assert_eq!(
            result.as_ref().err().unwrap().to_string(),
            "Command::Delay timed out or MPSC channel failed. Error: Timeout"
        );
    }
}
