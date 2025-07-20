//! The `lib_bench` module
//!
//! This module runs all the library benchmarks
use std::collections::HashMap;
use std::ffi::OsString;
use std::time::Instant;

use anyhow::Result;

use super::common::{Assistant, AssistantKind, Baselines, BenchmarkSummaries, Config, ModulePath};
use super::format::{LibraryBenchmarkHeader, OutputFormat};
use super::meta::Metadata;
use super::summary::{BaselineKind, BaselineName, BenchmarkKind, BenchmarkSummary, SummaryOutput};
use super::tool::config::ToolConfigs;
use super::tool::path::{ToolOutputPath, ToolOutputPathKind};
use super::tool::run::RunOptions;
use crate::api::{
    EntryPoint, LibraryBenchmarkBench, LibraryBenchmarkConfig, LibraryBenchmarkGroups, RawArgs,
    ValgrindTool,
};
use crate::error::Error;
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
    name: String,
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
    pub group_index: usize,
    pub bench_index: usize,
    pub id: Option<String>,
    pub function_name: String,
    pub args: Option<String>,
    pub run_options: RunOptions,
    pub tools: ToolConfigs,
    pub module_path: ModulePath,
    pub output_format: OutputFormat,
    pub default_tool: ValgrindTool,
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
    fn baselines(&self) -> Baselines;
    fn run(&self, lib_bench: &LibBench, config: &Config, group: &Group)
        -> Result<BenchmarkSummary>;
}

impl Benchmark for BaselineBenchmark {
    fn output_path(&self, lib_bench: &LibBench, config: &Config, group: &Group) -> ToolOutputPath {
        let kind = if lib_bench.default_tool.has_output_file() {
            ToolOutputPathKind::Out
        } else {
            ToolOutputPathKind::Log
        };
        ToolOutputPath::new(
            kind,
            lib_bench.default_tool,
            &self.baseline_kind,
            &config.meta.target_dir,
            &group.module_path,
            &lib_bench.name(),
        )
    }

    fn baselines(&self) -> Baselines {
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

        let out_path = self.output_path(lib_bench, config, group);
        out_path.init()?;

        for path in lib_bench.tools.output_paths(&out_path) {
            path.shift()?;
            if path.kind == ToolOutputPathKind::Out {
                path.to_log_output().shift()?;
            }
        }

        let benchmark_summary = lib_bench.create_benchmark_summary(
            config,
            &out_path,
            &lib_bench.function_name,
            header.description(),
            self.baselines(),
        )?;

        lib_bench.tools.run(
            &header.to_title(),
            benchmark_summary,
            &self.baselines(),
            &self.baseline_kind,
            config,
            &config.bench_bin,
            &lib_bench.bench_args(group),
            &lib_bench.run_options,
            &out_path,
            false,
            &lib_bench.module_path,
            &lib_bench.output_format,
        )
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
        let default_tool = benchmark_groups.default_tool;

        let mut groups = vec![];
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
                name: library_benchmark_group.id,
                module_path: group_module_path,
                benches: vec![],
                setup,
                teardown,
                compare_by_id: library_benchmark_group
                    .compare_by_id
                    .unwrap_or(defaults::COMPARE_BY_ID),
            };

            for (group_index, library_benchmark_benches) in library_benchmark_group
                .library_benchmarks
                .into_iter()
                .enumerate()
            {
                for (bench_index, library_benchmark_bench) in
                    library_benchmark_benches.benches.into_iter().enumerate()
                {
                    let config = group_config.clone().update_from_all([
                        library_benchmark_benches.config.as_ref(),
                        library_benchmark_bench.config.as_ref(),
                    ]);

                    let lib_bench = LibBench::new(
                        meta,
                        &group,
                        config,
                        group_index,
                        bench_index,
                        library_benchmark_bench,
                        default_tool,
                    )?;
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
                    .tools
                    .0
                    .iter()
                    .any(|c| c.regression_config.is_fail_fast());

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
    fn new(
        meta: &Metadata,
        group: &Group,
        config: LibraryBenchmarkConfig,
        group_index: usize,
        bench_index: usize,
        library_benchmark_bench: LibraryBenchmarkBench,
        default_tool: ValgrindTool,
    ) -> Result<Self> {
        let envs = config.resolve_envs();
        let mut default_args = HashMap::new();

        // The cachegrind client requests are not inserted into the benchmark function if the
        // default tool is not cachegrind, so setting --instr-at-start to `no` is only required if
        // the default tool sent by the benchmark harness (not with command-line arguments) is
        // cachegrind. Also, we only need to set this in library benchmarks, so it's best to use
        // `default_args` to add this command-line argument.
        let default_tool = if let Some(meta_default_tool) = meta.args.default_tool {
            meta_default_tool
        } else {
            if default_tool == ValgrindTool::Cachegrind {
                default_args.insert(
                    ValgrindTool::Cachegrind,
                    RawArgs::new(["--instr-at-start=no"]),
                );
            }
            config.default_tool.unwrap_or(default_tool)
        };

        let module_path = group
            .module_path
            .join(&library_benchmark_bench.function_name);

        let mut output_format = config
            .output_format
            .map_or_else(OutputFormat::default, Into::into);
        output_format.kind = meta.args.output_format;

        let tool_configs = ToolConfigs::new(
            &mut output_format,
            config.tools,
            &module_path,
            library_benchmark_bench.id.as_ref(),
            meta,
            default_tool,
            &EntryPoint::Default,
            &config.valgrind_args,
            &default_args,
        )
        .map_err(|error| {
            Error::ConfigurationError(
                module_path.clone(),
                library_benchmark_bench.id.clone(),
                error.to_string(),
            )
        })?;

        Ok(Self {
            group_index,
            bench_index,
            id: library_benchmark_bench.id,
            function_name: library_benchmark_bench.function_name,
            args: library_benchmark_bench.args,
            run_options: RunOptions {
                env_clear: config.env_clear.unwrap_or(true),
                envs,
                ..Default::default()
            },
            tools: tool_configs,
            module_path,
            output_format,
            default_tool,
        })
    }

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
            OsString::from(&group.name),
            OsString::from(self.group_index.to_string()),
            OsString::from(self.bench_index.to_string()),
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
        baselines: Baselines,
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
            baselines,
        ))
    }
}

impl Benchmark for LoadBaselineBenchmark {
    fn output_path(&self, lib_bench: &LibBench, config: &Config, group: &Group) -> ToolOutputPath {
        let kind = if lib_bench.default_tool.has_output_file() {
            ToolOutputPathKind::Base(self.loaded_baseline.to_string())
        } else {
            ToolOutputPathKind::BaseLog(self.loaded_baseline.to_string())
        };
        ToolOutputPath::new(
            kind,
            lib_bench.default_tool,
            &BaselineKind::Name(self.baseline.clone()),
            &config.meta.target_dir,
            &group.module_path,
            &lib_bench.name(),
        )
    }

    fn baselines(&self) -> Baselines {
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

        let out_path = self.output_path(lib_bench, config, group);

        let benchmark_summary = lib_bench.create_benchmark_summary(
            config,
            &out_path,
            &lib_bench.function_name,
            header.description(),
            self.baselines(),
        )?;

        lib_bench.tools.run_loaded_vs_base(
            &header.to_title(),
            &self.baseline,
            &self.loaded_baseline,
            benchmark_summary,
            &self.baselines(),
            config,
            &out_path,
            &lib_bench.output_format,
        )
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
        let kind = if lib_bench.default_tool.has_output_file() {
            ToolOutputPathKind::Base(self.baseline.to_string())
        } else {
            ToolOutputPathKind::BaseLog(self.baseline.to_string())
        };
        ToolOutputPath::new(
            kind,
            lib_bench.default_tool,
            &BaselineKind::Name(self.baseline.clone()),
            &config.meta.target_dir,
            &group.module_path,
            &lib_bench.name(),
        )
    }

    fn baselines(&self) -> Baselines {
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

        let out_path = self.output_path(lib_bench, config, group);
        out_path.init()?;

        let benchmark_summary = lib_bench.create_benchmark_summary(
            config,
            &out_path,
            &lib_bench.function_name,
            header.description(),
            self.baselines(),
        )?;

        lib_bench.tools.run(
            &header.to_title(),
            benchmark_summary,
            &self.baselines(),
            &BaselineKind::Name(self.baseline.clone()),
            config,
            &config.bench_bin,
            &lib_bench.bench_args(group),
            &lib_bench.run_options,
            &out_path,
            true,
            &lib_bench.module_path,
            &lib_bench.output_format,
        )
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
