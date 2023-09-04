use std::ffi::OsString;
use std::io::{stdin, Read};
use std::path::PathBuf;

use crate::api::{self, LibraryBenchmark, Options};
use crate::error::IaiCallgrindError;
use crate::runner::callgrind::{CallgrindArgs, CallgrindCommand, CallgrindOutput, Sentinel};
use crate::runner::meta::Metadata;
use crate::runner::print::Header;

#[derive(Debug)]
struct LibBench {
    bench_index: usize,
    index: usize,
    id: Option<String>,
    function: String,
    args: Option<String>,
    config: LibBenchConfig,
}

impl LibBench {
    fn run(&self, config: &Config, group: &GroupConfig) -> Result<(), IaiCallgrindError> {
        let command = CallgrindCommand::new(config.meta.aslr, &config.meta.arch);
        let (args, sentinel) = if let Some(group_id) = &group.id {
            (
                vec![
                    OsString::from("--iai-run".to_owned()),
                    OsString::from(group_id),
                    OsString::from(self.bench_index.to_string()),
                    OsString::from(self.index.to_string()),
                    OsString::from(format!("{}::{}", group.module, self.function)),
                ],
                Sentinel::from_segments([&config.module, &self.function, &self.function]),
            )
        } else {
            (
                vec![
                    OsString::from("--iai-run".to_owned()),
                    OsString::from(self.index.to_string()),
                    OsString::from(format!("{}::{}", group.module, self.function)),
                ],
                Sentinel::from_path(&config.module, &self.function),
            )
        };

        let mut callgrind_args = group.callgrind_args.clone();
        callgrind_args.insert_toggle_collect(&format!("*{}", sentinel.as_toggle()));

        let output = if let Some(bench_id) = &self.id {
            CallgrindOutput::create(
                &config.meta.target_dir,
                &group.module,
                &format!("{}.{}", &self.function, bench_id),
            )
        } else {
            CallgrindOutput::create(&config.meta.target_dir, &group.module, &self.function)
        };
        callgrind_args.set_output_file(&output.file);

        let options = Options {
            env_clear: self.config.env_clear,
            ..Default::default()
        };

        command.run(&callgrind_args, &config.executable, &args, vec![], &options)?;

        let new_stats = output.parse(&config.bench_file, &sentinel);

        let old_output = output.old_output();
        let old_stats = old_output
            .exists()
            .then(|| old_output.parse(&config.bench_file, sentinel));

        Header::from_segments(
            [&group.module, &self.function],
            self.id.clone(),
            self.args.clone(),
        )
        .print();

        new_stats.print(old_stats);

        Ok(())
    }
}

#[derive(Debug)]
struct LibBenchConfig {
    env_clear: bool,
}

#[derive(Debug)]
struct GroupConfig {
    id: Option<String>,
    benches: Vec<LibBench>,
    module: String,
    callgrind_args: CallgrindArgs,
}

#[derive(Debug)]
struct Config {
    #[allow(unused)]
    package_dir: PathBuf,
    bench_file: PathBuf,
    groups: Vec<GroupConfig>,
    executable: PathBuf,
    module: String,
    meta: Metadata,
}

impl Config {
    fn receive_benchmark(num_bytes: usize) -> Result<api::LibraryBenchmark, IaiCallgrindError> {
        let mut encoded = vec![];
        let mut stdin = stdin();
        stdin.read_to_end(&mut encoded).map_err(|error| {
            IaiCallgrindError::Other(format!("Failed to read encoded configuration: {error}"))
        })?;
        assert!(
            encoded.len() == num_bytes,
            "Bytes mismatch when decoding configuration: Expected {num_bytes} bytes but received: \
             {} bytes",
            encoded.len()
        );

        let benchmark: api::LibraryBenchmark = bincode::deserialize(&encoded).map_err(|error| {
            IaiCallgrindError::Other(format!("Failed to decode configuration: {error}"))
        })?;

        Ok(benchmark)
    }

    fn parse_benchmark(module: &str, benchmark: LibraryBenchmark) -> Vec<GroupConfig> {
        benchmark
            .groups
            .into_iter()
            .map(|group| {
                let module_path = if let Some(group_id) = &group.id {
                    format!("{module}::{group_id}")
                } else {
                    module.to_owned()
                };
                let config = if let Some(config) = group.config {
                    benchmark.config.clone().update(&config).clone()
                } else {
                    benchmark.config.clone()
                };
                let callgrind_args = {
                    let mut raw = config.raw_callgrind_args.0.clone();
                    raw.extend(benchmark.command_line_args.iter().cloned());

                    // The last argument is usually --bench. This argument comes from cargo and does
                    // not belong to the arguments passed from the main macro. So, we're removing it
                    // if it is there.
                    if raw.last().map_or(false, |a| a == "--bench") {
                        raw.pop();
                    }

                    CallgrindArgs::from_args(raw)
                };
                GroupConfig {
                    id: group.id,
                    module: module_path,
                    benches: group
                        .benches
                        .into_iter()
                        .enumerate()
                        .flat_map(|(bench_index, funcs)| {
                            funcs
                                .into_iter()
                                .enumerate()
                                .map(move |(index, f)| LibBench {
                                    bench_index,
                                    index,
                                    id: f.id,
                                    function: f.bench,
                                    args: f.args,
                                    config: LibBenchConfig {
                                        // TODO: default env_clear should be true
                                        env_clear: config.env_clear.unwrap_or_default(),
                                    },
                                })
                        })
                        .collect(),
                    callgrind_args,
                }
            })
            .collect::<Vec<GroupConfig>>()
    }

    fn generate(
        mut env_args_iter: impl Iterator<Item = OsString> + std::fmt::Debug,
    ) -> Result<Self, IaiCallgrindError> {
        let package_dir = PathBuf::from(env_args_iter.next().unwrap());
        let bench_file = PathBuf::from(env_args_iter.next().unwrap());
        let module = env_args_iter.next().unwrap().to_str().unwrap().to_owned();
        let executable = PathBuf::from(env_args_iter.next().unwrap());
        let num_bytes = env_args_iter
            .next()
            .unwrap()
            .to_string_lossy()
            .parse::<usize>()
            .unwrap();

        let benchmark = Self::receive_benchmark(num_bytes)?;
        let groups = Self::parse_benchmark(&module, benchmark);
        let meta = Metadata::new();

        Ok(Self {
            package_dir,
            bench_file,
            groups,
            executable,
            module,
            meta,
        })
    }
}

pub(crate) fn run(
    env_args: impl Iterator<Item = OsString> + std::fmt::Debug,
) -> Result<(), IaiCallgrindError> {
    let config = Config::generate(env_args)?;
    for group in &config.groups {
        for bench in &group.benches {
            bench.run(&config, group)?;
        }
    }
    Ok(())
}
