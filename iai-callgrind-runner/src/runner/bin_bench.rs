use std::io::stderr;
use std::path::PathBuf;
use std::process::Child;

use anyhow::Result;
use log::{debug, log_enabled, trace, Level};
use tempfile::TempDir;

use super::callgrind::args::Args;
use super::callgrind::flamegraph::{
    BaselineFlamegraphGenerator, Config as FlamegraphConfig, Flamegraph, FlamegraphGenerator,
    LoadBaselineFlamegraphGenerator, SaveBaselineFlamegraphGenerator,
};
use super::callgrind::summary_parser::SummaryParser;
use super::callgrind::{CallgrindCommand, RegressionConfig};
use super::common::{Assistant, AssistantKind, Config, ModulePath};
use super::format::{Header, OutputFormat, VerticalFormat};
use super::meta::Metadata;
use super::summary::{
    BaselineKind, BaselineName, BenchmarkKind, BenchmarkSummary, CallgrindSummary, CostsSummary,
    SummaryOutput,
};
use super::tool::{
    Parser, RunOptions, ToolConfigs, ToolOutputPath, ToolOutputPathKind, ValgrindTool,
};
use crate::api::{self, BinaryBenchmarkMain, Stdin};
use crate::error::Error;
use crate::runner::format::tool_headline;
use crate::util::{copy_directory, make_absolute, make_relative};

mod defaults {
    pub const SANDBOX_FIXTURES_FOLLOW_SYMLINKS: bool = false;
    pub const SANDBOX_ENABLED: bool = false;
    pub const REGRESSION_FAIL_FAST: bool = false;
    pub const ENV_CLEAR: bool = true;
}

#[derive(Debug)]
struct BaselineBenchmark {
    baseline_kind: BaselineKind,
}

#[derive(Debug)]
struct BinBench {
    id: Option<String>,
    args: Option<String>,
    function_name: String,
    command: api::Command,
    run_options: RunOptions,
    callgrind_args: Args,
    flamegraph_config: Option<FlamegraphConfig>,
    regression_config: Option<RegressionConfig>,
    tools: ToolConfigs,
    setup: Option<Assistant>,
    teardown: Option<Assistant>,
    sandbox: Option<api::Sandbox>,
    module_path: ModulePath,
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
struct Sandbox {
    current_dir: PathBuf,
    temp_dir: Option<TempDir>,
}

#[derive(Debug)]
struct SaveBaselineBenchmark {
    baseline: BaselineName,
}

trait Benchmark: std::fmt::Debug {
    fn output_path(&self, bin_bench: &BinBench, config: &Config, group: &Group) -> ToolOutputPath;
    fn baselines(&self) -> (Option<String>, Option<String>);
    fn run(
        &self,
        bin_bench: &BinBench,
        config: &Config,
        group: &Group,
        child: Option<Child>,
        setup: Option<&Assistant>,
    ) -> Result<BenchmarkSummary>;
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
        child: Option<Child>,
        setup: Option<&Assistant>,
    ) -> Result<BenchmarkSummary> {
        let callgrind_command = CallgrindCommand::new(&config.meta);

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

        let mut benchmark_summary = bin_bench.create_benchmark_summary(config, group, &out_path)?;

        let header = bin_bench.print_header(&config.meta, group);

        let output = callgrind_command.run(
            bin_bench.callgrind_args.clone(),
            &bin_bench.command.path,
            &bin_bench.command.args,
            bin_bench.run_options.clone(),
            &out_path,
            &bin_bench.module_path,
            child,
        )?;

        let new_costs = SummaryParser.parse(&out_path)?;

        #[allow(clippy::if_then_some_else_none)]
        let old_costs = if old_path.exists() {
            Some(SummaryParser.parse(&old_path)?)
        } else {
            None
        };

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
            &config.meta,
            &bin_bench.command.path,
            &bin_bench.command.args,
            &bin_bench.run_options,
            &out_path,
            false,
            &bin_bench.module_path,
        )?;

        Ok(benchmark_summary)
    }
}

impl BinBench {
    fn name(&self) -> String {
        if let Some(bench_id) = &self.id {
            format!("{}.{}", self.function_name, bench_id)
        } else {
            self.function_name.clone()
        }
    }

    fn print_header(&self, meta: &Metadata, group: &Group) -> Header {
        let path = make_relative(&meta.project_root, &self.command.path);

        let command_args: Vec<String> = self
            .command
            .args
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        let command_args = shlex::try_join(command_args.iter().map(String::as_str)).unwrap();

        let description = format!(
            "({}) -> {} {}",
            self.args.as_ref().map_or("", String::as_str),
            path.display(),
            command_args
        );

        let header = Header::from_module_path(
            &group.module_path.join(&self.function_name),
            self.id.clone(),
            description,
        );

        if meta.args.output_format == OutputFormat::Default {
            header.print();
            if self.tools.has_tools_enabled() {
                println!("{}", tool_headline(ValgrindTool::Callgrind));
            }
        }

        header
    }

    // TODO: DOUBLE CHECK. Just copied from lib_bench
    fn create_benchmark_summary(
        &self,
        config: &Config,
        group: &Group,
        output_path: &ToolOutputPath,
    ) -> Result<BenchmarkSummary> {
        let summary_output = if let Some(format) = config.meta.args.save_summary {
            let output = SummaryOutput::new(format, &output_path.dir);
            output.init()?;
            Some(output)
        } else {
            None
        };

        Ok(BenchmarkSummary::new(
            BenchmarkKind::LibraryBenchmark,
            config.meta.project_root.clone(),
            config.package_dir.clone(),
            config.bench_file.clone(),
            config.bench_bin.clone(),
            &group.module_path.join(&self.function_name),
            self.id.clone(),
            self.args.clone(),
            summary_output,
        ))
    }

    // TODO: DOUBLE CHECK. Just copied from lib_bench
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
        for bench in &self.benches {
            // We're implicitly applying the default here: In the absence of a user provided
            // sandbox we don't run the benchmarks in a sandbox.
            let sandbox = if let Some(sandbox) = &bench.sandbox {
                Some(Sandbox::setup(sandbox, &config.meta)?)
            } else {
                None
            };

            // The setup function runs within the sandbox
            let child = if let Some(setup) = &bench.setup {
                setup.run(
                    config,
                    &bench.id.as_ref().map_or_else(
                        || self.module_path.join(&bench.function_name),
                        |id| self.module_path.join(&bench.function_name).join(id),
                    ),
                    bench.command.stdin.as_ref().and_then(|s| {
                        if let Stdin::Setup(p) = s {
                            Some(p.clone())
                        } else {
                            None
                        }
                    }),
                )?
            } else {
                None
            };

            let fail_fast = bench
                .regression_config
                .as_ref()
                .map_or(defaults::REGRESSION_FAIL_FAST, |r| r.fail_fast);

            // TODO: Should we run the teardown in case of errors?
            let summary = benchmark.run(bench, config, self, child, bench.setup.as_ref())?;
            summary.print_and_save(&config.meta.args.output_format)?;

            // Likewise to the setup function, the teardown runs within the sandbox. Also, we run
            // the teardown function before any regression checks, just in case the regression
            // checks fail and we're returning with an error without having run the teardown.
            if let Some(teardown) = &bench.teardown {
                teardown.run(
                    config,
                    &bench.id.as_ref().map_or_else(
                        || self.module_path.join(&bench.function_name),
                        |id| self.module_path.join(&bench.function_name).join(id),
                    ),
                    None,
                )?;
            }

            summary.check_regression(is_regressed, fail_fast)?;

            if let Some(sandbox) = sandbox {
                sandbox.reset()?;
            }
        }

        Ok(())
    }
}

impl Groups {
    fn from_binary_benchmark(
        module: &ModulePath,
        benchmark: BinaryBenchmarkMain,
        meta: &Metadata,
    ) -> Result<Self> {
        // TODO: Mostly copied from lib_bench, DOUBLE_CHECK !!
        let global_config = benchmark.config;
        let meta_callgrind_args = meta.args.callgrind_args.clone().unwrap_or_default();

        let mut groups = vec![];
        for binary_benchmark_group in benchmark.groups {
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
            };

            // TODO: MORE OR LESS COPIED FROM lib_bench. Check if everything's right
            for (group_index, binary_benchmark_benches) in
                binary_benchmark_group.benches.into_iter().enumerate()
            {
                for (bench_index, binary_benchmark_bench) in
                    binary_benchmark_benches.benches.into_iter().enumerate()
                {
                    let command = binary_benchmark_bench.command;

                    let config = global_config.clone().update_from_all([
                        binary_benchmark_group.config.as_ref(),
                        binary_benchmark_benches.config.as_ref(),
                        binary_benchmark_bench.config.as_ref(),
                        Some(&command.config),
                    ]);

                    let envs = config.resolve_envs();

                    let callgrind_args =
                        Args::from_raw_args(&[&config.raw_callgrind_args, &meta_callgrind_args])?;
                    let flamegraph_config = config.flamegraph_config.map(Into::into);
                    let module_path = binary_benchmark_bench.id.as_ref().map_or_else(
                        || group.module_path.join(&binary_benchmark_bench.bench),
                        |id| {
                            group
                                .module_path
                                .join(&binary_benchmark_bench.bench)
                                .join(id)
                        },
                    );
                    let bin_bench = BinBench {
                        id: binary_benchmark_bench.id,
                        args: binary_benchmark_bench.args,
                        function_name: binary_benchmark_bench.bench,
                        // TODO: CHECK IF ALL OPTIONS ARE PASSED FROM COMMAND TO RunOptions
                        run_options: RunOptions {
                            env_clear: config.env_clear.unwrap_or(defaults::ENV_CLEAR),
                            entry_point: None,
                            envs,
                            stdin: command.stdin.clone(),
                            stdout: command.stdout.clone(),
                            stderr: command.stderr.clone(),
                            ..Default::default()
                        },
                        command,
                        callgrind_args,
                        flamegraph_config,
                        regression_config: api::update_option(
                            &config.regression_config,
                            &meta.regression_config,
                        )
                        .map(Into::into),
                        tools: ToolConfigs(config.tools.0.into_iter().map(Into::into).collect()),
                        setup: binary_benchmark_bench.has_setup.then_some(
                            Assistant::new_bench_assistant(
                                AssistantKind::Setup,
                                &group.name,
                                (group_index, bench_index),
                            ),
                        ),
                        teardown: binary_benchmark_bench.has_teardown.then_some(
                            Assistant::new_bench_assistant(
                                AssistantKind::Teardown,
                                &group.name,
                                (group_index, bench_index),
                            ),
                        ),
                        sandbox: config.sandbox,
                        module_path,
                    };
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
                setup.run(config, &group.module_path, None)?;
            }

            group.run(benchmark, &mut is_regressed, config)?;

            if let Some(teardown) = &group.teardown {
                teardown.run(config, &group.module_path, None)?;
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
        _child: Option<Child>,
        _setup: Option<&Assistant>,
    ) -> Result<BenchmarkSummary> {
        let out_path = self.output_path(bin_bench, config, group);
        let old_path = out_path.to_base_path();
        let log_path = out_path.to_log_output();
        let mut benchmark_summary = bin_bench.create_benchmark_summary(config, group, &out_path)?;

        let header = bin_bench.print_header(&config.meta, group);

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
    fn new(binary_benchmark: BinaryBenchmarkMain, config: Config) -> Result<Self> {
        let setup = binary_benchmark
            .has_setup
            .then_some(Assistant::new_main_assistant(AssistantKind::Setup));
        let teardown = binary_benchmark
            .has_teardown
            .then_some(Assistant::new_main_assistant(AssistantKind::Teardown));

        let groups =
            Groups::from_binary_benchmark(&config.module_path, binary_benchmark, &config.meta)?;

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
            setup.run(&self.config, &self.config.module_path, None)?;
        }

        self.groups.run(self.benchmark.as_ref(), &self.config)?;

        if let Some(teardown) = &self.teardown {
            teardown.run(&self.config, &self.config.module_path, None)?;
        }
        Ok(())
    }
}

impl Sandbox {
    fn setup(inner: &api::Sandbox, meta: &Metadata) -> Result<Self> {
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
                };
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

    fn reset(self) -> Result<()> {
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
        child: Option<Child>,
        setup: Option<&Assistant>,
    ) -> Result<BenchmarkSummary> {
        let callgrind_command = CallgrindCommand::new(&config.meta);
        let baselines = self.baselines();

        let out_path = self.output_path(bin_bench, config, group);
        out_path.init()?;

        #[allow(clippy::if_then_some_else_none)]
        let old_costs = if out_path.exists() {
            let old_costs = SummaryParser.parse(&out_path)?;
            out_path.clear()?;
            Some(old_costs)
        } else {
            None
        };

        let log_path = out_path.to_log_output();
        log_path.clear()?;

        let mut benchmark_summary = bin_bench.create_benchmark_summary(config, group, &out_path)?;

        let header = bin_bench.print_header(&config.meta, group);

        let output = callgrind_command.run(
            bin_bench.callgrind_args.clone(),
            &bin_bench.command.path,
            &bin_bench.command.args,
            bin_bench.run_options.clone(),
            &out_path,
            &bin_bench.module_path,
            child,
        )?;

        let new_costs = SummaryParser.parse(&out_path)?;
        let costs_summary = CostsSummary::new(&new_costs, old_costs.as_ref());
        VerticalFormat::default().print(&config.meta, baselines.clone(), &costs_summary)?;

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
            &config.meta,
            &bin_bench.command.path,
            &bin_bench.command.args,
            &bin_bench.run_options,
            &out_path,
            true,
            &bin_bench.module_path,
        )?;

        Ok(benchmark_summary)
    }
}

pub fn run(binary_benchmark: BinaryBenchmarkMain, config: Config) -> Result<()> {
    Runner::new(binary_benchmark, config)?.run()
}
