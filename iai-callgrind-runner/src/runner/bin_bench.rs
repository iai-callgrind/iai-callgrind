use std::collections::HashMap;
use std::io::stderr;

use anyhow::{anyhow, Result};

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
use crate::api::{self, BinaryBenchmarkBench, BinaryBenchmarkConfig, BinaryBenchmarkGroups, Stdin};
use crate::error::Error;
use crate::runner::format;

mod defaults {
    pub const REGRESSION_FAIL_FAST: bool = false;
    pub const ENV_CLEAR: bool = true;
    pub const COMPARE_BY_ID: bool = false;
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
    pub command: api::Command,
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

        let header = BinaryBenchmarkHeader::new(&config.meta, bin_bench);
        let mut benchmark_summary =
            bin_bench.create_benchmark_summary(config, &out_path, header.description())?;

        header.print();

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

        let command = binary_benchmark_bench.command;
        if command.path.display().to_string().is_empty() {
            return Err(anyhow!("{module_path}: Empty path in command",));
        }

        let envs = config.resolve_envs();

        let callgrind_args = Args::from_raw_args(&[&config.raw_callgrind_args, raw_args])?;
        let flamegraph_config = config.flamegraph_config.map(Into::into);

        Ok(Self {
            id: binary_benchmark_bench.id,
            args: binary_benchmark_bench.args,
            function_name: binary_benchmark_bench.function_name,
            run_options: RunOptions {
                env_clear: config.env_clear.unwrap_or(defaults::ENV_CLEAR),
                envs,
                stdin: command.stdin.clone(),
                stdout: command.stdout.clone(),
                stderr: command.stderr.clone(),
                exit_with: config.exit_with,
                current_dir: config.current_dir,
            },
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
                    command.stdin.as_ref().and_then(|s| {
                        if let Stdin::Setup(p) = s {
                            Some(p.clone())
                        } else {
                            None
                        }
                    }),
                )),
            teardown: binary_benchmark_bench.has_teardown.then_some(
                Assistant::new_bench_assistant(
                    AssistantKind::Teardown,
                    &group.name,
                    (group_index, bench_index),
                    None,
                ),
            ),
            sandbox: config.sandbox,
            module_path,
            command,
            truncate_description: config.truncate_description.unwrap_or(Some(50)),
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

            let setup = binary_benchmark_group
                .has_setup
                .then_some(Assistant::new_group_assistant(
                    AssistantKind::Setup,
                    &binary_benchmark_group.id,
                ));
            let teardown =
                binary_benchmark_group
                    .has_teardown
                    .then_some(Assistant::new_group_assistant(
                        AssistantKind::Teardown,
                        &binary_benchmark_group.id,
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
                    let config = global_config.clone().update_from_all([
                        binary_benchmark_group.config.as_ref(),
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
        let out_path = self.output_path(bin_bench, config, group);
        let old_path = out_path.to_base_path();
        let log_path = out_path.to_log_output();

        let header = BinaryBenchmarkHeader::new(&config.meta, bin_bench);
        let mut benchmark_summary =
            bin_bench.create_benchmark_summary(config, &out_path, header.description())?;
        header.print();

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
            .then_some(Assistant::new_main_assistant(AssistantKind::Setup));
        let teardown = benchmark_groups
            .has_teardown
            .then_some(Assistant::new_main_assistant(AssistantKind::Teardown));

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

        let header = BinaryBenchmarkHeader::new(&config.meta, bin_bench);
        let mut benchmark_summary =
            bin_bench.create_benchmark_summary(config, &out_path, header.description())?;

        header.print();

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
