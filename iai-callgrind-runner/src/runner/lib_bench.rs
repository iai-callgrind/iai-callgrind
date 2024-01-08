//! The `lib_bench` module
//!
//! This module runs all the library benchmarks
use std::ffi::OsString;
use std::io::stderr;

use anyhow::Result;

use super::callgrind::args::Args;
use super::callgrind::flamegraph::{
    BaselineFlamegraphGenerator, Config as FlamegraphConfig, Flamegraph, FlamegraphGenerator,
    LoadBaselineFlamegraphGenerator, SaveBaselineFlamegraphGenerator,
};
use super::callgrind::parser::Sentinel;
use super::callgrind::sentinel_parser::SentinelParser;
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
use super::{Config, Error};
use crate::api::{self, LibraryBenchmark};

/// Implements [`Benchmark`] to run a [`LibBench`] and compare against a earlier [`BenchmarkKind`]
#[derive(Debug)]
struct BaselineBenchmark {
    baseline_kind: BaselineKind,
}

// A `Group` is the organizational unit and counterpart of the `library_benchmark_group!` macro
#[derive(Debug)]
struct Group {
    id: Option<String>,
    benches: Vec<LibBench>,
    module: String,
}

/// `Groups` is the top-level organizational unit of the `main!` macro for library benchmarks
#[derive(Debug)]
struct Groups(Vec<Group>);

/// A `LibBench` represents a single benchmark under the `#[library_benchmark]` attribute macro
///
/// It needs an implementation of `Benchmark` to be run.
#[derive(Debug)]
struct LibBench {
    bench_index: usize,
    index: usize,
    id: Option<String>,
    function: String,
    args: Option<String>,
    options: RunOptions,
    callgrind_args: Args,
    flamegraph_config: Option<FlamegraphConfig>,
    regression_config: Option<RegressionConfig>,
    tools: ToolConfigs,
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
            &group.module,
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
        let callgrind_command = CallgrindCommand::new(&config.meta);
        let bench_args = lib_bench.bench_args(group);

        let sentinel = Sentinel::new("iai_callgrind::bench::");
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

        let mut benchmark_summary = lib_bench.create_benchmark_summary(config, group, &out_path)?;

        let header = lib_bench.print_header(&config.meta, group);

        let output = callgrind_command.run(
            lib_bench.callgrind_args.clone(),
            &config.bench_bin,
            &bench_args,
            lib_bench.options.clone(),
            &out_path,
        )?;

        let new_costs = SentinelParser::new(&sentinel).parse(&out_path)?;

        #[allow(clippy::if_then_some_else_none)]
        let old_costs = if old_path.exists() {
            Some(SentinelParser::new(&sentinel).parse(&old_path)?)
        } else {
            None
        };

        let costs_summary = CostsSummary::new(&new_costs, old_costs.as_ref());
        VerticalFormat::default().print(&config.meta, self.baselines(), &costs_summary)?;

        output.dump_log(log::Level::Info);
        log_path.dump_log(log::Level::Info, &mut stderr())?;

        let regressions = lib_bench.check_and_print_regressions(&costs_summary);

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                log_path.real_paths()?,
                out_path.real_paths()?,
            ));

        callgrind_summary.add_summary(
            &config.bench_bin,
            &bench_args,
            &old_path,
            costs_summary,
            regressions,
        );

        if let Some(flamegraph_config) = lib_bench.flamegraph_config.clone() {
            callgrind_summary.flamegraphs = BaselineFlamegraphGenerator {
                baseline_kind: self.baseline_kind.clone(),
            }
            .create(
                &Flamegraph::new(header.to_title(), flamegraph_config),
                &out_path,
                Some(&sentinel),
                &config.meta.project_root,
            )?;
        }

        benchmark_summary.tool_summaries = lib_bench.tools.run(
            &config.meta,
            &config.bench_bin,
            &bench_args,
            &lib_bench.options,
            &out_path,
        )?;

        Ok(benchmark_summary)
    }
}

impl Groups {
    /// Create this `Groups` from a [`LibraryBenchmark`] submitted by the benchmarking harness
    fn from_library_benchmark(
        module: &str,
        benchmark: LibraryBenchmark,
        meta: &Metadata,
    ) -> Result<Self> {
        let global_config = benchmark.config;
        let mut groups = vec![];
        let meta_callgrind_args = meta.args.callgrind_args.clone().unwrap_or_default();

        for library_benchmark_group in benchmark.groups {
            let module_path = if let Some(group_id) = &library_benchmark_group.id {
                format!("{module}::{group_id}")
            } else {
                module.to_owned()
            };
            let mut group = Group {
                id: library_benchmark_group.id,
                module: module_path,
                benches: vec![],
            };
            for (bench_index, library_benchmark_benches) in
                library_benchmark_group.benches.into_iter().enumerate()
            {
                for (index, library_benchmark_bench) in
                    library_benchmark_benches.benches.into_iter().enumerate()
                {
                    let config = global_config.clone().update_from_all([
                        library_benchmark_group.config.as_ref(),
                        library_benchmark_benches.config.as_ref(),
                        library_benchmark_bench.config.as_ref(),
                    ]);
                    let envs = config.resolve_envs();
                    let callgrind_args =
                        Args::from_raw_args(&[&config.raw_callgrind_args, &meta_callgrind_args])?;
                    let flamegraph_config = config.flamegraph_config.map(Into::into);
                    let lib_bench = LibBench {
                        bench_index,
                        index,
                        id: library_benchmark_bench.id,
                        function: library_benchmark_bench.bench,
                        args: library_benchmark_bench.args,
                        options: RunOptions {
                            env_clear: config.env_clear.unwrap_or(true),
                            entry_point: Some("iai_callgrind::bench::*".to_owned()),
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
                        tools: ToolConfigs(config.tools.0.into_iter().map(Into::into).collect()),
                    };
                    group.benches.push(lib_bench);
                }
            }
            groups.push(group);
        }

        Ok(Self(groups))
    }

    /// Run all [`LibBench`] benchmarks
    fn run(&self, benchmark: &dyn Benchmark, config: &Config) -> Result<()> {
        let mut is_regressed = false;

        for group in &self.0 {
            for bench in &group.benches {
                let fail_fast = bench
                    .regression_config
                    .as_ref()
                    .map_or(false, |r| r.fail_fast);
                let summary = benchmark.run(bench, config, group)?;
                summary.print_and_save(&config.meta.args.output_format)?;
                summary.check_regression(&mut is_regressed, fail_fast)?;
            }
        }

        if is_regressed {
            Err(Error::RegressionError(false).into())
        } else {
            Ok(())
        }
    }
}

impl LibBench {
    /// The name of this `LibBench` consisting of the name of the benchmark function and the id of
    /// the bench attribute (`#[bench::ID(...)]`)
    ///
    /// The name is whenever it is necessary to identify a benchmark run within the same
    /// [`Group`].
    fn name(&self) -> String {
        if let Some(bench_id) = &self.id {
            format!("{}.{}", &self.function, bench_id)
        } else {
            self.function.clone()
        }
    }

    /// The arguments for the `bench_bin` to actually run the benchmark function
    ///
    /// Not all [`Group`]s have an id
    fn bench_args(&self, group: &Group) -> Vec<OsString> {
        if let Some(group_id) = &group.id {
            vec![
                OsString::from("--iai-run".to_owned()),
                OsString::from(group_id),
                OsString::from(self.bench_index.to_string()),
                OsString::from(self.index.to_string()),
                OsString::from(format!("{}::{}", group.module, self.function)),
            ]
        } else {
            vec![
                OsString::from("--iai-run".to_owned()),
                OsString::from(self.index.to_string()),
                OsString::from(format!("{}::{}", group.module, self.function)),
            ]
        }
    }

    /// This method creates the initial [`BenchmarkSummary`]
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
            &[&group.module, &self.function],
            self.id.clone(),
            self.args.clone(),
            summary_output,
        ))
    }

    /// Print the headline of the terminal output for this benchmark run
    ///
    /// If there are more tools than the usual callgrind run, this method also prints the tool
    /// summary header
    fn print_header(&self, meta: &Metadata, group: &Group) -> Header {
        let header = Header::from_segments(
            [&group.module, &self.function],
            self.id.clone(),
            self.args.clone(),
        );

        if meta.args.output_format == OutputFormat::Default {
            header.print();
            if self.tools.has_tools_enabled() {
                println!("{}", tool_headline(ValgrindTool::Callgrind));
            }
        }
        header
    }

    /// Check for regressions as defined in [`RegressionConfig`] and print an error if a regression
    /// occurred
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
}

impl Benchmark for LoadBaselineBenchmark {
    fn output_path(&self, lib_bench: &LibBench, config: &Config, group: &Group) -> ToolOutputPath {
        ToolOutputPath::new(
            ToolOutputPathKind::Base(self.loaded_baseline.to_string()),
            ValgrindTool::Callgrind,
            &BaselineKind::Name(self.baseline.clone()),
            &config.meta.target_dir,
            &group.module,
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
        let bench_args = lib_bench.bench_args(group);
        let sentinel = Sentinel::new("iai_callgrind::bench::");
        let out_path = self.output_path(lib_bench, config, group);
        let old_path = out_path.to_base_path();
        let log_path = out_path.to_log_output();
        let mut benchmark_summary = lib_bench.create_benchmark_summary(config, group, &out_path)?;

        let header = lib_bench.print_header(&config.meta, group);

        let new_costs = SentinelParser::new(&sentinel).parse(&out_path)?;
        let old_costs = Some(SentinelParser::new(&sentinel).parse(&old_path)?);
        let costs_summary = CostsSummary::new(&new_costs, old_costs.as_ref());

        VerticalFormat::default().print(&config.meta, self.baselines(), &costs_summary)?;

        let regressions = lib_bench.check_and_print_regressions(&costs_summary);

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                log_path.real_paths()?,
                out_path.real_paths()?,
            ));

        callgrind_summary.add_summary(
            &config.bench_bin,
            &bench_args,
            &old_path,
            costs_summary,
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
                Some(&sentinel),
                &config.meta.project_root,
            )?;
        }

        benchmark_summary.tool_summaries = lib_bench
            .tools
            .run_loaded_vs_base(&config.meta, &out_path)?;

        Ok(benchmark_summary)
    }
}

impl Runner {
    /// Create a new `Runner`
    fn new(library_benchmark: LibraryBenchmark, config: Config) -> Result<Self> {
        let groups =
            Groups::from_library_benchmark(&config.module, library_benchmark, &config.meta)?;

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
        })
    }

    /// Run all benchmarks in all groups
    fn run(&self) -> Result<()> {
        self.groups.run(self.benchmark.as_ref(), &self.config)
    }
}

impl Benchmark for SaveBaselineBenchmark {
    fn output_path(&self, lib_bench: &LibBench, config: &Config, group: &Group) -> ToolOutputPath {
        ToolOutputPath::new(
            ToolOutputPathKind::Base(self.baseline.to_string()),
            ValgrindTool::Callgrind,
            &BaselineKind::Name(self.baseline.clone()),
            &config.meta.target_dir,
            &group.module,
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
        let callgrind_command = CallgrindCommand::new(&config.meta);
        let bench_args = lib_bench.bench_args(group);
        let baselines = self.baselines();

        let sentinel = Sentinel::new("iai_callgrind::bench::");
        let out_path = self.output_path(lib_bench, config, group);
        out_path.init()?;

        #[allow(clippy::if_then_some_else_none)]
        let old_costs = if out_path.exists() {
            let old_costs = SentinelParser::new(&sentinel).parse(&out_path)?;
            out_path.clear()?;
            Some(old_costs)
        } else {
            None
        };

        let log_path = out_path.to_log_output();
        log_path.clear()?;

        for path in lib_bench.tools.output_paths(&out_path) {
            path.clear()?;
            path.to_log_output().clear()?;
        }

        let mut benchmark_summary = lib_bench.create_benchmark_summary(config, group, &out_path)?;

        let header = lib_bench.print_header(&config.meta, group);

        let output = callgrind_command.run(
            lib_bench.callgrind_args.clone(),
            &config.bench_bin,
            &bench_args,
            lib_bench.options.clone(),
            &out_path,
        )?;

        let new_costs = SentinelParser::new(&sentinel).parse(&out_path)?;
        let costs_summary = CostsSummary::new(&new_costs, old_costs.as_ref());
        VerticalFormat::default().print(&config.meta, baselines.clone(), &costs_summary)?;

        output.dump_log(log::Level::Info);
        log_path.dump_log(log::Level::Info, &mut stderr())?;

        let regressions = lib_bench.check_and_print_regressions(&costs_summary);

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                log_path.real_paths()?,
                out_path.real_paths()?,
            ));

        callgrind_summary.add_summary(
            &config.bench_bin,
            &bench_args,
            &out_path,
            costs_summary,
            regressions,
        );

        if let Some(flamegraph_config) = lib_bench.flamegraph_config.clone() {
            callgrind_summary.flamegraphs = SaveBaselineFlamegraphGenerator {
                baseline: self.baseline.clone(),
            }
            .create(
                &Flamegraph::new(header.to_title(), flamegraph_config),
                &out_path,
                Some(&sentinel),
                &config.meta.project_root,
            )?;
        }

        benchmark_summary.tool_summaries = lib_bench.tools.run(
            &config.meta,
            &config.bench_bin,
            &bench_args,
            &lib_bench.options,
            &out_path,
        )?;

        Ok(benchmark_summary)
    }
}

/// The top-level method which should be used to initiate running all benchmarks
pub fn run(library_benchmark: LibraryBenchmark, config: Config) -> Result<()> {
    Runner::new(library_benchmark, config)?.run()
}
