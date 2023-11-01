use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::Result;

use super::callgrind::args::Args;
use super::callgrind::flamegraph::{Config as FlamegraphConfig, Flamegraph};
use super::callgrind::parser::{Parser, Sentinel};
use super::callgrind::sentinel_parser::SentinelParser;
use super::callgrind::{CallgrindCommand, CallgrindOptions, CallgrindOutput, Regression};
use super::meta::Metadata;
use super::print::{Formatter, Header, VerticalFormat};
use super::Error;
use crate::api::{self, LibraryBenchmark, RawCallgrindArgs};
use crate::util::receive_benchmark;

#[derive(Debug)]
struct Config {
    #[allow(unused)]
    package_dir: PathBuf,
    #[allow(unused)]
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
    opts: CallgrindOptions,
    callgrind_args: Args,
    flamegraph: Option<FlamegraphConfig>,
    regression: Option<Regression>,
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
        let command_line_args =
            RawCallgrindArgs::from_command_line_args(benchmark.command_line_args);

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
                    let callgrind_args = Args::from_raw_callgrind_args(&[
                        &config.raw_callgrind_args,
                        &command_line_args,
                    ])?;
                    let flamegraph = config.flamegraph.map(std::convert::Into::into);
                    let lib_bench = LibBench {
                        bench_index,
                        index,
                        id: library_benchmark_bench.id,
                        function: library_benchmark_bench.bench,
                        args: library_benchmark_bench.args,
                        opts: CallgrindOptions {
                            env_clear: config.env_clear.unwrap_or(true),
                            entry_point: Some("iai_callgrind::bench::*".to_owned()),
                            envs,
                            ..Default::default()
                        },
                        callgrind_args,
                        flamegraph,
                        regression: api::update_option(&config.regression, &meta.regression_config)
                            .map(std::convert::Into::into),
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
                if let Err(error) = bench.run(config, group) {
                    // We catch the regression error here and return immediately it if it is fatal.
                    // Else, we return the regression error later which let's the main process fail
                    // but in a non-fatal way
                    if let Some(Error::RegressionError(false)) = error.downcast_ref::<Error>() {
                        is_regressed = true;
                    } else {
                        return Err(error);
                    }
                }
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
    fn run(&self, config: &Config, group: &Group) -> Result<()> {
        let command = CallgrindCommand::new(&config.meta);
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
        let output = if let Some(bench_id) = &self.id {
            CallgrindOutput::init(
                &config.meta.target_dir,
                &group.module,
                &format!("{}.{}", &self.function, bench_id),
            )
        } else {
            CallgrindOutput::init(&config.meta.target_dir, &group.module, &self.function)
        };

        command.run(
            self.callgrind_args.clone(),
            &config.bench_bin,
            &args,
            self.opts.clone(),
            &output,
        )?;

        let header = Header::from_segments(
            [&group.module, &self.function],
            self.id.clone(),
            self.args.clone(),
        );

        if let Some(flamegraph_config) = self.flamegraph.clone() {
            Flamegraph::new(header.to_title(), flamegraph_config).create(
                &output,
                Some(&sentinel),
                &config.meta.project_root,
            )?;
        }

        let new_costs = SentinelParser::new(&sentinel).parse(&output)?;

        let old_output = output.to_old_output();

        #[allow(clippy::if_then_some_else_none)]
        let old_costs = if old_output.exists() {
            Some(SentinelParser::new(&sentinel).parse(&old_output)?)
        } else {
            None
        };

        header.print();
        let string = VerticalFormat::default().format(&new_costs, old_costs.as_ref())?;
        print!("{string}");

        if let Some(regression) = &self.regression {
            regression.check_and_print(&new_costs, old_costs.as_ref())?;
        }

        Ok(())
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

        let benchmark = receive_benchmark(num_bytes)?;
        let meta = Metadata::new()?;
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
