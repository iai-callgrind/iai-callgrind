use std::ffi::OsString;
use std::io::stdout;

use anyhow::Result;

use super::callgrind::args::Args;
use super::callgrind::flamegraph::{Config as FlamegraphConfig, Flamegraph};
use super::callgrind::parser::{Parser, Sentinel};
use super::callgrind::sentinel_parser::SentinelParser;
use super::callgrind::{CallgrindCommand, Regression};
use super::meta::Metadata;
use super::print::{Formatter, Header, VerticalFormat};
use super::summary::{BaselineName, CallgrindRegressionSummary};
use super::tool::{RunOptions, ToolConfigs};
use super::{Config, Error};
use crate::api::{self, LibraryBenchmark};
use crate::runner::print::tool_summary_header;
use crate::runner::summary::{
    BaselineKind, BenchmarkKind, BenchmarkSummary, CallgrindSummary, CostsSummary, SummaryOutput,
};
use crate::runner::tool::{ToolOutputPath, ToolOutputPathKind, ValgrindTool};

#[derive(Debug)]
struct BaselineLibBenchRunner {
    baseline_kind: BaselineKind,
}

#[derive(Debug)]
struct BenchmarkRunnerFactory;

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

/// A `LibBench` represents a single benchmark from the `#[library_benchmark]` attribute macro
#[derive(Debug)]
struct LibBench {
    bench_index: usize,
    index: usize,
    id: Option<String>,
    function: String,
    args: Option<String>,
    options: RunOptions,
    callgrind_args: Args,
    flamegraph: Option<FlamegraphConfig>,
    regression: Option<Regression>,
    tools: ToolConfigs,
}

#[derive(Debug)]
struct LoadedBaselineLibBenchRunner {
    loaded_baseline: BaselineName,
    baseline: BaselineName,
}

#[derive(Debug)]
struct Runner {
    config: Config,
    groups: Groups,
}

#[derive(Debug)]
struct SaveBaselineLibBenchRunner {
    baseline: BaselineName,
}

trait LibBenchRunner: std::fmt::Debug {
    fn output_path(&self, lib_bench: &LibBench, config: &Config, group: &Group) -> ToolOutputPath;
    fn baselines(&self) -> (Option<String>, Option<String>);
    fn run(&self, lib_bench: &LibBench, config: &Config, group: &Group)
    -> Result<BenchmarkSummary>;
}

impl LibBenchRunner for BaselineLibBenchRunner {
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
        out_path.init();
        out_path.shift();

        let old_path = out_path.to_base_path();
        let log_path = out_path.to_log_output();
        log_path.shift();

        for path in lib_bench.tools.output_paths(&out_path) {
            path.shift();
            path.to_log_output().shift();
        }

        let mut benchmark_summary = lib_bench.create_benchmark_summary(config, group, &out_path);

        let header = lib_bench.print_header(group);

        let output = callgrind_command.run(
            lib_bench.callgrind_args.clone(),
            &config.bench_bin,
            &bench_args,
            lib_bench.options.clone(),
            &out_path,
        )?;

        let new_costs = SentinelParser::new(&sentinel).parse(&out_path)?;

        // TODO: BASELINE if not Old MUST EXIST OR ERROR
        #[allow(clippy::if_then_some_else_none)]
        let old_costs = if old_path.exists() {
            Some(SentinelParser::new(&sentinel).parse(&old_path)?)
        } else {
            None
        };

        let costs_summary = CostsSummary::new(&new_costs, old_costs.as_ref());
        print!(
            "{}",
            VerticalFormat::default().format(self.baselines(), &costs_summary)?
        );

        output.dump_log(log::Level::Info);
        log_path.dump_log(log::Level::Info, &mut stdout())?;

        let regressions = lib_bench.check_and_print_regressions(&costs_summary);
        let fail_fast = lib_bench.regression.as_ref().map_or(false, |r| r.fail_fast);

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                fail_fast,
                log_path.real_paths(),
                out_path.real_paths(),
            ));

        callgrind_summary.add_summary(
            &config.bench_bin,
            &bench_args,
            &old_path,
            costs_summary,
            regressions,
        );

        if let Some(flamegraph_config) = lib_bench.flamegraph.clone() {
            callgrind_summary.flamegraphs = Flamegraph::new(header.to_title(), flamegraph_config)
                .create(
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

impl BenchmarkRunnerFactory {
    fn create_runner(
        save_baseline: Option<&BaselineName>,
        load_baseline: Option<&BaselineName>,
        baseline: Option<&BaselineName>,
    ) -> Box<dyn LibBenchRunner> {
        if let Some(baseline_name) = save_baseline {
            Box::new(SaveBaselineLibBenchRunner {
                baseline: baseline_name.clone(),
            })
        } else if let Some(baseline_name) = load_baseline {
            Box::new(LoadedBaselineLibBenchRunner {
                loaded_baseline: baseline_name.clone(),
                baseline: baseline.unwrap().clone(),
            })
        } else {
            Box::new(BaselineLibBenchRunner {
                baseline_kind: baseline
                    .map_or(BaselineKind::Old, |name| BaselineKind::Name(name.clone())),
            })
        }
    }
}

impl Groups {
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
                    let flamegraph = config.flamegraph.map(Into::into);
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
                        flamegraph,
                        regression: api::update_option(&config.regression, &meta.regression_config)
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

    fn run(&self, config: &Config) -> Result<()> {
        let runner = BenchmarkRunnerFactory::create_runner(
            config.meta.args.save_baseline.as_ref(),
            config.meta.args.load_baseline.as_ref(),
            config.meta.args.baseline.as_ref(),
        );
        let mut is_regressed = false;

        for group in &self.0 {
            for bench in &group.benches {
                let summary = runner.run(bench, config, group)?;
                summary.save()?;
                summary.check_regression(&mut is_regressed)?;
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
    fn name(&self) -> String {
        if let Some(bench_id) = &self.id {
            format!("{}.{}", &self.function, bench_id)
        } else {
            self.function.clone()
        }
    }

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

    fn create_benchmark_summary(
        &self,
        config: &Config,
        group: &Group,
        output_path: &ToolOutputPath,
    ) -> BenchmarkSummary {
        let summary_output = config.meta.args.save_summary.map(|format| {
            let output = SummaryOutput::new(format, &output_path.dir);
            output.init();
            output
        });
        BenchmarkSummary::new(
            BenchmarkKind::LibraryBenchmark,
            config.meta.project_root.clone(),
            config.package_dir.clone(),
            config.bench_file.clone(),
            config.bench_bin.clone(),
            &[&group.module, &self.function],
            self.id.clone(),
            self.args.clone(),
            summary_output,
        )
    }

    fn print_header(&self, group: &Group) -> Header {
        let header = Header::from_segments(
            [&group.module, &self.function],
            self.id.clone(),
            self.args.clone(),
        );

        header.print();
        if self.tools.has_tools_enabled() {
            println!("{}", tool_summary_header(ValgrindTool::Callgrind));
        }
        header
    }

    fn check_and_print_regressions(
        &self,
        costs_summary: &CostsSummary,
    ) -> Vec<CallgrindRegressionSummary> {
        if let Some(regression) = &self.regression {
            regression.check_and_print(costs_summary)
        } else {
            vec![]
        }
    }
}

impl LibBenchRunner for LoadedBaselineLibBenchRunner {
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
        let mut benchmark_summary = lib_bench.create_benchmark_summary(config, group, &out_path);

        lib_bench.print_header(group);

        let new_costs = SentinelParser::new(&sentinel).parse(&out_path)?;
        let old_costs = Some(SentinelParser::new(&sentinel).parse(&old_path)?);
        let costs_summary = CostsSummary::new(&new_costs, old_costs.as_ref());

        print!(
            "{}",
            VerticalFormat::default().format(self.baselines(), &costs_summary)?
        );

        let regressions = lib_bench.check_and_print_regressions(&costs_summary);
        let fail_fast = lib_bench.regression.as_ref().map_or(false, |r| r.fail_fast);

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                fail_fast,
                log_path.real_paths(),
                out_path.real_paths(),
            ));

        callgrind_summary.add_summary(
            &config.bench_bin,
            &bench_args,
            &old_path,
            costs_summary,
            regressions,
        );

        benchmark_summary.tool_summaries = lib_bench
            .tools
            .run_loaded_vs_base(&config.meta, &out_path)?;

        Ok(benchmark_summary)
    }
}

impl Runner {
    fn generate(library_benchmark: LibraryBenchmark, config: Config) -> Result<Self> {
        let groups =
            Groups::from_library_benchmark(&config.module, library_benchmark, &config.meta)?;

        Ok(Self { config, groups })
    }

    fn run(&self) -> Result<()> {
        self.groups.run(&self.config)
    }
}

impl LibBenchRunner for SaveBaselineLibBenchRunner {
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

    // TODO: MOVE TO LibBenchRunner
    // fn initialize(&self, lib_bench: &LibBench, config: &Config, group: &Group) ->
    // (ToolOutputPath, ToolOutputPath) {
    //
    // }

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
        out_path.init();

        #[allow(clippy::if_then_some_else_none)]
        let old_costs = if out_path.exists() {
            let old_costs = SentinelParser::new(&sentinel).parse(&out_path)?;
            out_path.clear();
            Some(old_costs)
        } else {
            None
        };

        let log_path = out_path.to_log_output();
        log_path.clear();

        for path in lib_bench.tools.output_paths(&out_path) {
            path.clear();
            path.to_log_output().clear();
        }

        let mut benchmark_summary = lib_bench.create_benchmark_summary(config, group, &out_path);

        lib_bench.print_header(group);

        let output = callgrind_command.run(
            lib_bench.callgrind_args.clone(),
            &config.bench_bin,
            &bench_args,
            lib_bench.options.clone(),
            &out_path,
        )?;

        let new_costs = SentinelParser::new(&sentinel).parse(&out_path)?;
        let costs_summary = CostsSummary::new(&new_costs, old_costs.as_ref());
        let string = VerticalFormat::default().format(self.baselines(), &costs_summary)?;
        print!("{string}");

        output.dump_log(log::Level::Info);
        log_path.dump_log(log::Level::Info, &mut stdout())?;

        let regressions = lib_bench.check_and_print_regressions(&costs_summary);
        let fail_fast = lib_bench.regression.as_ref().map_or(false, |r| r.fail_fast);

        let callgrind_summary = benchmark_summary
            .callgrind_summary
            .insert(CallgrindSummary::new(
                fail_fast,
                log_path.real_paths(),
                out_path.real_paths(),
            ));

        callgrind_summary.add_summary(
            &config.bench_bin,
            &bench_args,
            &out_path,
            costs_summary,
            regressions,
        );

        // TODO: MAKE THIS WORK
        // if let Some(flamegraph_config) = self.flamegraph.clone() {
        //     callgrind_summary.flamegraphs = Flamegraph::new(header.to_title(), flamegraph_config)
        //         .create(
        //         &output_path,
        //         Some(&sentinel),
        //         &config.meta.project_root,
        //     )?;
        // }

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

pub fn run(library_benchmark: LibraryBenchmark, config: Config) -> Result<()> {
    Runner::generate(library_benchmark, config)?.run()
}
