use std::ffi::OsString;
use std::path::PathBuf;

// TODO: Remove all Callgrind prefixes for structs contained in the callgrind module??
use super::callgrind::args::CallgrindArgs;
use super::callgrind::flamegraph_parser::FlamegraphParser;
use super::callgrind::parser::CallgrindParser;
use super::callgrind::sentinel_parser::SentinelParser;
use super::callgrind::{CallgrindCommand, CallgrindOptions, CallgrindOutput, Sentinel};
use super::flamegraph::{Flamegraph, FlamegraphOutput};
use super::meta::Metadata;
use super::print::Header;
use crate::api::LibraryBenchmark;
use crate::error::Result;
use crate::util::receive_benchmark;

/// A `LibBench` represents a single benchmark from the `#[library_benchmark]` attribute macro
#[derive(Debug)]
struct LibBench {
    bench_index: usize,
    index: usize,
    id: Option<String>,
    function: String,
    args: Option<String>,
    opts: CallgrindOptions,
    callgrind_args: CallgrindArgs,
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
            CallgrindOutput::create(
                &config.meta.target_dir,
                &group.module,
                &format!("{}.{}", &self.function, bench_id),
            )
        } else {
            CallgrindOutput::create(&config.meta.target_dir, &group.module, &self.function)
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

        let flamegraph_parser = FlamegraphParser::new(Some(&sentinel), &config.meta.project_root);

        flamegraph_parser.parse(&output).and_then(|stacks| {
            FlamegraphOutput::create(&output).and_then(|flamegraph_output| {
                let flamegraph = Flamegraph {
                    stacks,
                    // TODO: This field should be part of a FlamegraphOptions (own not from inferno)
                    title: header.to_title(),
                };
                flamegraph.create(&flamegraph_output)
            })
        })?;

        let new_stats = SentinelParser::new(&sentinel, &config.bench_file).parse(&output)?;

        let old_output = output.old_output();

        #[allow(clippy::if_then_some_else_none)]
        let old_stats = if old_output.exists() {
            Some(SentinelParser::new(&sentinel, &config.bench_file).parse(&old_output)?)
        } else {
            None
        };

        header.print();
        new_stats.print(old_stats);

        Ok(())
    }
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

impl Groups {
    fn from_library_benchmark(module: &str, benchmark: LibraryBenchmark) -> Result<Self> {
        let global_config = &benchmark.config;
        let mut groups = vec![];
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
                    let callgrind_args = {
                        let mut raw = config.raw_callgrind_args;
                        raw.extend_from_command_line_args(benchmark.command_line_args.as_slice());
                        CallgrindArgs::from_raw_callgrind_args(&raw)?
                    };
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
                    };
                    group.benches.push(lib_bench);
                }
            }
            groups.push(group);
        }

        Ok(Self(groups))
    }

    fn run(&self, config: &Config) -> Result<()> {
        for group in &self.0 {
            for bench in &group.benches {
                bench.run(config, group)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct Config {
    #[allow(unused)]
    package_dir: PathBuf,
    bench_file: PathBuf,
    #[allow(unused)]
    module: String,
    bench_bin: PathBuf,
    meta: Metadata,
}

#[derive(Debug)]
struct Runner {
    config: Config,
    groups: Groups,
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
        let groups = Groups::from_library_benchmark(&module, benchmark)?;
        let meta = Metadata::new()?;

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
