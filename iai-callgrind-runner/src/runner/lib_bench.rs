//! The `lib_bench` module
//!
//! This module runs all the library benchmarks
use std::collections::HashMap;
use std::ffi::OsString;
use std::io::stderr;
use std::time::Instant;

use anyhow::Result;

use super::callgrind::args::Args;
use super::callgrind::flamegraph::{
    BaselineFlamegraphGenerator, Config as FlamegraphConfig, Flamegraph, FlamegraphGenerator,
    LoadBaselineFlamegraphGenerator, SaveBaselineFlamegraphGenerator,
};
use super::callgrind::parser::{CallgrindParser, Sentinel};
use super::callgrind::summary_parser::SummaryParser;
use super::callgrind::{RegressionConfig, Summaries};
use super::common::{Assistant, AssistantKind, BenchmarkSummaries, Config, ModulePath};
use super::format::{
    print_no_capture_footer, Formatter, LibraryBenchmarkHeader, OutputFormat, VerticalFormatter,
};
use super::meta::Metadata;
use super::summary::{
    BaselineKind, BaselineName, BenchmarkKind, BenchmarkSummary, CallgrindRegression,
    CallgrindSummary, MetricsSummary, SummaryOutput, ToolRun,
};
use super::tool::{
    RunOptions, ToolCommand, ToolConfig, ToolConfigs, ToolOutputPath, ToolOutputPathKind,
    ValgrindTool,
};
use super::DEFAULT_TOGGLE;
use crate::api::{self, EntryPoint, LibraryBenchmarkGroups};
use crate::runner::format;

mod defaults {
    pub const COMPARE_BY_ID: bool = false;
}

/// Implements [`Benchmark`] to run a [`LibBench`] and compare against an earlier [`BenchmarkKind`]
#[derive(Debug)]
struct BaselineBenchmark {
    baseline_kind: BaselineKind,
}

// A `Group` is the organizational unit and counterpart of the `library_benchmark_group!` macro
#[derive(Debug)]
struct Group {
    id: String,
    benches: Vec<LibBench>,
    compare_by_id: bool,
    module_path: ModulePath,
    setup: Option<Assistant>,
    teardown: Option<Assistant>,
}

/// `Groups` is the top-level organizational unit of the `main!` macro for library benchmarks
#[derive(Debug)]
struct Groups(Vec<Group>);

/// A `LibBench` represents a single benchmark under the `#[library_benchmark]` attribute macro
///
/// It needs an implementation of `Benchmark` to be run.
#[derive(Debug)]
pub struct LibBench {
    pub bench_index: usize,
    pub index: usize,
    pub id: Option<String>,
    pub function_name: String,
    pub args: Option<String>,
    pub run_options: RunOptions,
    pub callgrind_args: Args,
    pub flamegraph_config: Option<FlamegraphConfig>,
    pub regression_config: Option<RegressionConfig>,
    pub tools: ToolConfigs,
    pub module_path: ModulePath,
    pub entry_point: EntryPoint,
    pub output_format: OutputFormat,
}

/// Implements [`Benchmark`] to load a [`LibBench`] baseline run and compare against another
/// baseline
///
/// This benchmark runner does not run valgrind or execute anything.
#[derive(Debug)]
struct LoadBaselineBenchmark {
    loaded_baseline: BaselineName,
    baseline: BaselineName,
}

/// Create and run [`Groups`] with an implementation of [`Benchmark`]
#[derive(Debug)]
struct Runner {
    config: Config,
    groups: Groups,
    benchmark: Box<dyn Benchmark>,
    setup: Option<Assistant>,
    teardown: Option<Assistant>,
}

/// Implements [`Benchmark`] to save a [`LibBench`] run as baseline. If present compare against a
/// former baseline with the same name
#[derive(Debug)]
struct SaveBaselineBenchmark {
    baseline: BaselineName,
}

/// This trait needs to be implemented to actually run a [`LibBench`]
///
/// Despite having the same name, this trait differs from `bin_bench::Benchmark` and is
/// designed to run a `LibBench` only.
trait Benchmark: std::fmt::Debug {
    fn output_path(&self, lib_bench: &LibBench, config: &Config, group: &Group) -> ToolOutputPath;
    fn baselines(&self) -> (Option<String>, Option<String>);
    fn run(&self, lib_bench: &LibBench, config: &Config, group: &Group)
        -> Result<BenchmarkSummary>;
}

impl Benchmark for BaselineBenchmark {
    fn output_path(&self, lib_bench: &LibBench, config: &Config, group: &Group) -> ToolOutputPath {
        ToolOutputPath::new(
            ToolOutputPathKind::Out,
            ValgrindTool::Callgrind,
            &self.baseline_kind,
            &config.meta.target_dir,
            &group.module_path,
            &lib_bench.name(),
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
        lib_bench: &LibBench,
        config: &Config,
        group: &Group,
    ) -> Result<BenchmarkSummary> {
        let header = LibraryBenchmarkHeader::new(lib_bench);
        header.print();

        let callgrind_command = ToolCommand::new(
            ValgrindTool::Callgrind,
            &config.meta,
            config.meta.args.nocapture,
        );

        let mut callgrind_args = lib_bench.callgrind_args.clone();

        match &lib_bench.entry_point {
            EntryPoint::None => {}
            EntryPoint::Default => {
                callgrind_args.insert_toggle_collect(DEFAULT_TOGGLE);
            }
            EntryPoint::Custom(custom) => {
                callgrind_args.insert_toggle_collect(custom);
            }
        }

        let tool_config = ToolConfig::new(ValgrindTool::Callgrind, true, callgrind_args, None);

        let bench_args = lib_bench.bench_args(group);

        let out_path = self.output_path(lib_bench, config, group);
        out_path.init()?;
        out_path.shift()?;

        let old_path = out_path.to_base_path();
        let log_path = out_path.to_log_output();
        log_path.shift()?;

        for path in lib_bench.tools.output_paths(&out_path) {
            path.shift()?;
            path.to_log_output().shift()?;
        }

        let mut benchmark_summary = lib_bench.create_benchmark_summary(
            config,
            &out_path,
            &lib_bench.function_name,
            header.description(),
        )?;

        let output = callgrind_command.run(
            tool_config,
            &config.bench_bin,
            &bench_args,
            lib_bench.run_options.clone(),
            &out_path,
            &lib_bench.module_path,
            None,
        )?;

        print_no_capture_footer(
            config.meta.args.nocapture,
            lib_bench.run_options.stdout.as_ref(),
            lib_bench.run_options.stderr.as_ref(),
        );

        let parser = SummaryParser;
        let parsed_new = parser.parse(&out_path)?;
        let parsed_old = old_path
            .exists()
            .then(|| parser.parse(&old_path))
            .transpose()?;

        let summaries = Summaries::new(parsed_new, parsed_old);

        VerticalFormatter::new(lib_bench.output_format.clone()).print(
            config,
            self.baselines(),
            &ToolRun::from(&summaries),
        )?;

        output.dump_log(log::Level::Info);
        log_path.dump_log(log::Level::Info, &mut stderr())?;

        let regressions = lib_bench.check_and_print_regressions(&summaries.total);

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                log_path.real_paths()?,
                out_path.real_paths()?,
            ));

        callgrind_summary.add_summaries(
            &config.bench_bin,
            &bench_args,
            &self.baselines(),
            summaries,
            regressions,
        );

        if let Some(flamegraph_config) = lib_bench.flamegraph_config.clone() {
            callgrind_summary.flamegraphs = BaselineFlamegraphGenerator {
                baseline_kind: self.baseline_kind.clone(),
            }
            .create(
                &Flamegraph::new(header.to_title(), flamegraph_config),
                &out_path,
                (lib_bench.entry_point == EntryPoint::Default)
                    .then(Sentinel::default)
                    .as_ref(),
                &config.meta.project_root,
            )?;
        }

        benchmark_summary.tool_summaries = lib_bench.tools.run(
            config,
            &config.bench_bin,
            &bench_args,
            &lib_bench.run_options,
            &out_path,
            false,
            &lib_bench.module_path,
            None,
            None,
            None,
            None,
            &lib_bench.output_format,
        )?;

        Ok(benchmark_summary)
    }
}

impl Groups {
    /// Create this `Groups` from a [`crate::api::LibraryBenchmark`] submitted by the benchmarking
    /// harness
    fn from_library_benchmark(
        module_path: &ModulePath,
        benchmark_groups: LibraryBenchmarkGroups,
        meta: &Metadata,
    ) -> Result<Self> {
        let global_config = benchmark_groups.config;
        let mut groups = vec![];
        let meta_callgrind_args = meta.args.callgrind_args.clone().unwrap_or_default();

        for library_benchmark_group in benchmark_groups.groups {
            let group_module_path = module_path.join(&library_benchmark_group.id);
            let group_config = global_config
                .clone()
                .update_from_all([library_benchmark_group.config.as_ref()]);

            let setup =
                library_benchmark_group
                    .has_setup
                    .then_some(Assistant::new_group_assistant(
                        AssistantKind::Setup,
                        &library_benchmark_group.id,
                        group_config.collect_envs(),
                        false,
                    ));
            let teardown =
                library_benchmark_group
                    .has_teardown
                    .then_some(Assistant::new_group_assistant(
                        AssistantKind::Teardown,
                        &library_benchmark_group.id,
                        group_config.collect_envs(),
                        false,
                    ));

            let mut group = Group {
                id: library_benchmark_group.id,
                module_path: group_module_path.clone(),
                compare_by_id: library_benchmark_group
                    .compare_by_id
                    .unwrap_or(defaults::COMPARE_BY_ID),
                benches: vec![],
                setup,
                teardown,
            };

            for (bench_index, library_benchmark_benches) in library_benchmark_group
                .library_benchmarks
                .into_iter()
                .enumerate()
            {
                for (index, library_benchmark_bench) in
                    library_benchmark_benches.benches.into_iter().enumerate()
                {
                    let config = group_config.clone().update_from_all([
                        library_benchmark_benches.config.as_ref(),
                        library_benchmark_bench.config.as_ref(),
                    ]);
                    let envs = config.resolve_envs();

                    let callgrind_args = Args::try_from_raw_args(&[
                        &config.valgrind_args,
                        &config.callgrind_args,
                        &meta_callgrind_args,
                    ])?;

                    let flamegraph_config = config.flamegraph_config.map(Into::into);
                    let module_path =
                        group_module_path.join(&library_benchmark_bench.function_name);

                    let mut output_format = config
                        .output_format
                        .map_or_else(OutputFormat::default, Into::into);
                    output_format.kind = meta.args.output_format;

                    let lib_bench = LibBench {
                        bench_index,
                        index,
                        id: library_benchmark_bench.id,
                        function_name: library_benchmark_bench.function_name,
                        args: library_benchmark_bench.args,
                        entry_point: config.entry_point.unwrap_or(EntryPoint::Default),
                        run_options: RunOptions {
                            env_clear: config.env_clear.unwrap_or(true),
                            envs,
                            ..Default::default()
                        },
                        callgrind_args,
                        flamegraph_config,
                        regression_config: api::update_option(
                            &config.regression_config,
                            &meta.regression_config,
                        )
                        .map(Into::into),
                        tools: ToolConfigs(
                            config
                                .tools
                                .0
                                .into_iter()
                                .map(|mut t| {
                                    if !config.valgrind_args.is_empty() {
                                        let mut new_args = config.valgrind_args.clone();
                                        new_args.extend_ignore_flag(t.raw_args.0.iter());
                                        t.raw_args = new_args;
                                    }
                                    t.try_into()
                                })
                                .collect::<Result<Vec<_>, _>>()?,
                        ),
                        module_path,
                        output_format,
                    };
                    group.benches.push(lib_bench);
                }
            }

            groups.push(group);
        }

        Ok(Self(groups))
    }

    /// Run all [`LibBench`] benchmarks
    fn run(&self, benchmark: &dyn Benchmark, config: &Config) -> Result<BenchmarkSummaries> {
        let mut benchmark_summaries = BenchmarkSummaries::default();
        for group in &self.0 {
            if let Some(setup) = &group.setup {
                setup.run(config, &group.module_path)?;
            }

            let mut lib_bench_summaries: HashMap<String, Vec<BenchmarkSummary>> =
                HashMap::with_capacity(group.benches.len());
            for bench in &group.benches {
                let fail_fast = bench
                    .regression_config
                    .as_ref()
                    .is_some_and(|r| r.fail_fast);

                let lib_bench_summary = benchmark.run(bench, config, group)?;
                lib_bench_summary.print_and_save(&config.meta.args.output_format)?;
                lib_bench_summary.check_regression(fail_fast)?;

                benchmark_summaries.add_summary(lib_bench_summary.clone());
                if group.compare_by_id && bench.output_format.is_default() {
                    if let Some(id) = &lib_bench_summary.id {
                        if let Some(sums) = lib_bench_summaries.get_mut(id) {
                            for sum in sums.iter() {
                                sum.compare_and_print(
                                    id,
                                    &lib_bench_summary,
                                    &bench.output_format,
                                )?;
                            }
                            sums.push(lib_bench_summary);
                        } else {
                            lib_bench_summaries.insert(id.clone(), vec![lib_bench_summary]);
                        }
                    }
                }
            }

            if let Some(teardown) = &group.teardown {
                teardown.run(config, &group.module_path)?;
            }
        }

        Ok(benchmark_summaries)
    }
}

impl LibBench {
    /// The name of this `LibBench` consisting of the name of the benchmark function and if present,
    /// the id of the bench attribute (`#[bench::ID(...)]`)
    ///
    /// The name is used to identify a benchmark run within the same [`Group`] and has therefore to
    /// be unique within the same [`Group`]
    fn name(&self) -> String {
        if let Some(bench_id) = &self.id {
            format!("{}.{}", &self.function_name, bench_id)
        } else {
            self.function_name.clone()
        }
    }

    /// The arguments for the `bench_bin` to actually run the benchmark function
    fn bench_args(&self, group: &Group) -> Vec<OsString> {
        vec![
            OsString::from("--iai-run".to_owned()),
            OsString::from(&group.id),
            OsString::from(self.bench_index.to_string()),
            OsString::from(self.index.to_string()),
            OsString::from(self.module_path.to_string()),
        ]
    }

    /// This method creates the initial [`BenchmarkSummary`]
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
            BenchmarkKind::LibraryBenchmark,
            config.meta.project_root.clone(),
            config.package_dir.clone(),
            config.bench_file.clone(),
            config.bench_bin.clone(),
            &self.module_path,
            function_name,
            self.id.clone(),
            description,
            summary_output,
        ))
    }

    /// Check for regressions as defined in [`RegressionConfig`] and print an error if a regression
    /// occurred
    fn check_and_print_regressions(
        &self,
        metrics_summary: &MetricsSummary,
    ) -> Vec<CallgrindRegression> {
        if let Some(regression_config) = &self.regression_config {
            regression_config.check_and_print(metrics_summary)
        } else {
            vec![]
        }
    }
}

impl Benchmark for LoadBaselineBenchmark {
    fn output_path(&self, lib_bench: &LibBench, config: &Config, group: &Group) -> ToolOutputPath {
        ToolOutputPath::new(
            ToolOutputPathKind::Base(self.loaded_baseline.to_string()),
            ValgrindTool::Callgrind,
            &BaselineKind::Name(self.baseline.clone()),
            &config.meta.target_dir,
            &group.module_path,
            &lib_bench.name(),
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
        lib_bench: &LibBench,
        config: &Config,
        group: &Group,
    ) -> Result<BenchmarkSummary> {
        let header = LibraryBenchmarkHeader::new(lib_bench);
        header.print();

        let bench_args = lib_bench.bench_args(group);
        let out_path = self.output_path(lib_bench, config, group);
        let old_path = out_path.to_base_path();
        let log_path = out_path.to_log_output();

        let mut benchmark_summary = lib_bench.create_benchmark_summary(
            config,
            &out_path,
            &lib_bench.function_name,
            header.description(),
        )?;

        let parser = SummaryParser;
        let parsed_new = parser.parse(&out_path)?;
        let parsed_old = Some(parser.parse(&old_path)?);
        let summaries = Summaries::new(parsed_new, parsed_old);

        VerticalFormatter::new(lib_bench.output_format.clone()).print(
            config,
            self.baselines(),
            &ToolRun::from(&summaries),
        )?;

        let regressions = lib_bench.check_and_print_regressions(&summaries.total);

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                log_path.real_paths()?,
                out_path.real_paths()?,
            ));

        callgrind_summary.add_summaries(
            &config.bench_bin,
            &bench_args,
            &self.baselines(),
            summaries,
            regressions,
        );

        if let Some(flamegraph_config) = lib_bench.flamegraph_config.clone() {
            callgrind_summary.flamegraphs = LoadBaselineFlamegraphGenerator {
                loaded_baseline: self.loaded_baseline.clone(),
                baseline: self.baseline.clone(),
            }
            .create(
                &Flamegraph::new(header.to_title(), flamegraph_config),
                &out_path,
                (lib_bench.entry_point == EntryPoint::Default)
                    .then(Sentinel::default)
                    .as_ref(),
                &config.meta.project_root,
            )?;
        }

        benchmark_summary.tool_summaries =
            lib_bench
                .tools
                .run_loaded_vs_base(config, &out_path, &lib_bench.output_format)?;

        Ok(benchmark_summary)
    }
}

impl Runner {
    /// Create a new `Runner`
    fn new(benchmark_groups: LibraryBenchmarkGroups, config: Config) -> Result<Self> {
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
            Groups::from_library_benchmark(&config.module_path, benchmark_groups, &config.meta)?;

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
            config,
            groups,
            benchmark,
            setup,
            teardown,
        })
    }

    /// Run all benchmarks in all groups
    fn run(&self) -> Result<BenchmarkSummaries> {
        if let Some(setup) = &self.setup {
            setup.run(&self.config, &self.config.module_path)?;
        }

        let summaries = self.groups.run(self.benchmark.as_ref(), &self.config)?;

        if let Some(teardown) = &self.teardown {
            teardown.run(&self.config, &self.config.module_path)?;
        }

        Ok(summaries)
    }
}

impl Benchmark for SaveBaselineBenchmark {
    fn output_path(&self, lib_bench: &LibBench, config: &Config, group: &Group) -> ToolOutputPath {
        ToolOutputPath::new(
            ToolOutputPathKind::Base(self.baseline.to_string()),
            ValgrindTool::Callgrind,
            &BaselineKind::Name(self.baseline.clone()),
            &config.meta.target_dir,
            &group.module_path,
            &lib_bench.name(),
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
        lib_bench: &LibBench,
        config: &Config,
        group: &Group,
    ) -> Result<BenchmarkSummary> {
        let header = LibraryBenchmarkHeader::new(lib_bench);
        header.print();

        let callgrind_command = ToolCommand::new(
            ValgrindTool::Callgrind,
            &config.meta,
            config.meta.args.nocapture,
        );

        let mut callgrind_args = lib_bench.callgrind_args.clone();
        match &lib_bench.entry_point {
            EntryPoint::None => {}
            EntryPoint::Default => {
                callgrind_args.insert_toggle_collect(DEFAULT_TOGGLE);
            }
            EntryPoint::Custom(custom) => {
                callgrind_args.insert_toggle_collect(custom);
            }
        }

        let tool_config = ToolConfig::new(ValgrindTool::Callgrind, true, callgrind_args, None);

        let bench_args = lib_bench.bench_args(group);

        let out_path = self.output_path(lib_bench, config, group);
        out_path.init()?;

        let parser = SummaryParser;
        let parsed_old = out_path
            .exists()
            .then(|| {
                parser
                    .parse(&out_path)
                    .and_then(|parsed| out_path.clear().map(|()| parsed))
            })
            .transpose()?;

        let log_path = out_path.to_log_output();
        log_path.clear()?;

        let mut benchmark_summary = lib_bench.create_benchmark_summary(
            config,
            &out_path,
            &lib_bench.function_name,
            header.description(),
        )?;

        let output = callgrind_command.run(
            tool_config,
            &config.bench_bin,
            &bench_args,
            lib_bench.run_options.clone(),
            &out_path,
            &lib_bench.module_path,
            None,
        )?;

        print_no_capture_footer(
            config.meta.args.nocapture,
            lib_bench.run_options.stdout.as_ref(),
            lib_bench.run_options.stderr.as_ref(),
        );

        let parsed_new = parser.parse(&out_path)?;
        let summaries = Summaries::new(parsed_new, parsed_old);

        VerticalFormatter::new(lib_bench.output_format.clone()).print(
            config,
            self.baselines(),
            &ToolRun::from(&summaries),
        )?;

        output.dump_log(log::Level::Info);
        log_path.dump_log(log::Level::Info, &mut stderr())?;

        let regressions = lib_bench.check_and_print_regressions(&summaries.total);

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                log_path.real_paths()?,
                out_path.real_paths()?,
            ));

        callgrind_summary.add_summaries(
            &config.bench_bin,
            &bench_args,
            &self.baselines(),
            summaries,
            regressions,
        );

        if let Some(flamegraph_config) = lib_bench.flamegraph_config.clone() {
            callgrind_summary.flamegraphs = SaveBaselineFlamegraphGenerator {
                baseline: self.baseline.clone(),
            }
            .create(
                &Flamegraph::new(header.to_title(), flamegraph_config),
                &out_path,
                (lib_bench.entry_point == EntryPoint::Default)
                    .then(Sentinel::default)
                    .as_ref(),
                &config.meta.project_root,
            )?;
        }

        benchmark_summary.tool_summaries = lib_bench.tools.run(
            config,
            &config.bench_bin,
            &bench_args,
            &lib_bench.run_options,
            &out_path,
            true,
            &lib_bench.module_path,
            // We don't have a sandbox feature in library benchmarks
            None,
            None,
            None,
            None,
            &lib_bench.output_format,
        )?;

        Ok(benchmark_summary)
    }
}

/// The top-level method which should be used to initiate running all benchmarks
pub fn run(benchmark_groups: LibraryBenchmarkGroups, config: Config) -> Result<BenchmarkSummaries> {
    let runner = Runner::new(benchmark_groups, config)?;

    let start = Instant::now();
    let mut summaries = runner.run()?;
    summaries.elapsed(start);

    Ok(summaries)
}

/// Print a list of all benchmarks with a short summary
pub fn list(benchmark_groups: LibraryBenchmarkGroups, config: &Config) -> Result<()> {
    let groups =
        Groups::from_library_benchmark(&config.module_path, benchmark_groups, &config.meta)?;

    let mut sum = 0u64;
    for group in groups.0 {
        for bench in group.benches {
            sum += 1;
            format::print_list_benchmark(&bench.module_path, bench.id.as_ref());
        }
    }

    format::print_benchmark_list_summary(sum);

    Ok(())
}
