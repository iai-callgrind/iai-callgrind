use std::ffi::OsString;
use std::io::stdout;
use std::path::PathBuf;

use anyhow::Result;

use super::callgrind::args::Args;
use super::callgrind::flamegraph::{Config as FlamegraphConfig, Flamegraph};
use super::callgrind::parser::{Parser, Sentinel};
use super::callgrind::sentinel_parser::SentinelParser;
use super::callgrind::{CallgrindCommand, Regression};
use super::meta::Metadata;
use super::print::{Formatter, Header, VerticalFormat};
use super::tool::{RunOptions, ToolConfigs};
use super::Error;
use crate::api::{self, LibraryBenchmark};
use crate::runner::print::tool_summary_header;
use crate::runner::summary::{
    BenchmarkKind, BenchmarkSummary, CallgrindSummary, CostsSummary, SummaryOutput,
};
use crate::runner::tool::{ToolOutputPath, ValgrindTool};
use crate::util::receive_benchmark;

#[derive(Debug)]
struct Config {
    package_dir: PathBuf,
    bench_file: PathBuf,
    #[allow(unused)]
    module: String,
    bench_bin: PathBuf,
    meta: Metadata,
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
struct Runner {
    config: Config,
    groups: Groups,
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
        let mut is_regressed = false;
        for group in &self.0 {
            for bench in &group.benches {
                let summary = bench.run(config, group)?;
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
    #[allow(clippy::too_many_lines)]
    fn run(&self, config: &Config, group: &Group) -> Result<BenchmarkSummary> {
        let callgrind_command = CallgrindCommand::new(&config.meta);
        let args = if let Some(group_id) = &group.id {
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
        };

        let sentinel = Sentinel::new("iai_callgrind::bench::");
        let output_path = if let Some(bench_id) = &self.id {
            ToolOutputPath::with_init(
                ValgrindTool::Callgrind,
                &config.meta.target_dir,
                &group.module,
                &format!("{}.{}", &self.function, bench_id),
            )
        } else {
            ToolOutputPath::with_init(
                ValgrindTool::Callgrind,
                &config.meta.target_dir,
                &group.module,
                &self.function,
            )
        };

        let log_path = output_path.to_log_output();
        log_path.init();

        let summary_output = config.meta.args.save_summary.map(|format| {
            let output = SummaryOutput::new(format, &output_path.dir);
            output.init();
            output
        });

        let mut benchmark_summary = BenchmarkSummary::new(
            BenchmarkKind::LibraryBenchmark,
            config.meta.project_root.clone(),
            config.package_dir.clone(),
            config.bench_file.clone(),
            config.bench_bin.clone(),
            &[&group.module, &self.function],
            self.id.clone(),
            self.args.clone(),
            summary_output,
        );

        let header = Header::from_segments(
            [&group.module, &self.function],
            self.id.clone(),
            self.args.clone(),
        );

        header.print();
        if self.tools.has_tools_enabled() {
            println!("{}", tool_summary_header(ValgrindTool::Callgrind));
        }

        let output = callgrind_command.run(
            self.callgrind_args.clone(),
            &config.bench_bin,
            &args,
            self.options.clone(),
            &output_path,
        )?;

        let new_costs = SentinelParser::new(&sentinel).parse(&output_path)?;

        let old_output = output_path.to_old_output();
        #[allow(clippy::if_then_some_else_none)]
        let old_costs = if old_output.exists() {
            Some(SentinelParser::new(&sentinel).parse(&old_output)?)
        } else {
            None
        };

        // TODO: Make use of CostsSummary
        let string = VerticalFormat::default().format(&new_costs, old_costs.as_ref())?;
        print!("{string}");

        output.dump_log(log::Level::Info);
        log_path.dump_log(log::Level::Info, &mut stdout())?;

        let (regressions, fail_fast) = if let Some(regression) = &self.regression {
            // TODO: Make use of CostsSummary
            (
                regression.check_and_print(&new_costs, old_costs.as_ref()),
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
            &args,
            &old_output,
            CostsSummary::new(&new_costs, old_costs.as_ref()),
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
            &args,
            &self.options,
            &output_path,
        )?;

        Ok(benchmark_summary)
    }
}

impl Runner {
    fn generate<I>(mut env_args_iter: I) -> Result<Self>
    where
        I: Iterator<Item = OsString> + std::fmt::Debug,
    {
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

        let benchmark: LibraryBenchmark = receive_benchmark(num_bytes)?;
        let meta = Metadata::new(&benchmark.command_line_args)?;
        let groups = Groups::from_library_benchmark(&module, benchmark, &meta)?;

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
