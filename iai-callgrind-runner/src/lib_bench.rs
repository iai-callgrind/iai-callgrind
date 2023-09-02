use std::ffi::OsString;
use std::io::{stdin, Read};
use std::path::PathBuf;

use crate::api::{self, LibraryBenchmarkGroup, Options};
use crate::callgrind::{CallgrindArgs, CallgrindCommand, CallgrindOutput, Sentinel};
use crate::meta::Metadata;
use crate::print::Header;
use crate::IaiCallgrindError;

#[derive(Debug)]
struct LibBench {
    bench_index: usize,
    index: usize,
    id: Option<String>,
    function: String,
    args: Option<String>,
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

        let mut callgrind_args = config.callgrind_args.clone();
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

        // TODO: env_clear should be true
        let options = Options {
            env_clear: false,
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
struct GroupConfig {
    id: Option<String>,
    benches: Vec<LibBench>,
    module: String,
}

#[derive(Debug)]
struct Config {
    #[allow(unused)]
    package_dir: PathBuf,
    bench_file: PathBuf,
    groups: Vec<GroupConfig>,
    executable: PathBuf,
    module: String,
    callgrind_args: CallgrindArgs,
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

    fn parse_groups(module: &str, groups: &[LibraryBenchmarkGroup]) -> Vec<GroupConfig> {
        groups
            .iter()
            .map(|group| {
                let module_path = if let Some(group_id) = &group.id {
                    format!("{module}::{group_id}")
                } else {
                    module.to_owned()
                };
                GroupConfig {
                    id: group.id.clone(),
                    module: module_path,
                    benches: group
                        .benches
                        .iter()
                        .enumerate()
                        .flat_map(|(bench_index, funcs)| {
                            funcs.iter().enumerate().map(move |(index, f)| LibBench {
                                bench_index,
                                index,
                                id: f.id.clone(),
                                function: f.bench.clone(),
                                args: f.args.clone(),
                            })
                        })
                        .collect(),
                }
            })
            .collect::<Vec<GroupConfig>>()
    }

    fn parse_callgrind_args(value: &[String]) -> CallgrindArgs {
        let mut callgrind_args: Vec<OsString> = value.iter().map(OsString::from).collect();

        // The last argument is sometimes --bench. This argument comes from cargo and does not
        // belong to the arguments passed from the main macro. So, we're removing it if it is there.
        if callgrind_args.last().map_or(false, |a| a == "--bench") {
            callgrind_args.pop();
        }

        CallgrindArgs::from_args(&callgrind_args)
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
        let groups = Self::parse_groups(&module, &benchmark.groups);
        let callgrind_args = Self::parse_callgrind_args(&benchmark.config.raw_callgrind_args.0);
        let meta = Metadata::new();

        Ok(Self {
            package_dir,
            bench_file,
            groups,
            executable,
            module,
            callgrind_args,
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
