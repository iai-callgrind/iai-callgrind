//! The module responsible for running a binary benchmark
//! TODO: IMPLEMENT iter

mod defaults {
    use crate::api::Stdin;

    pub const COMPARE_BY_ID: bool = false;
    pub const ENV_CLEAR: bool = true;
    pub const STDIN: Stdin = Stdin::Pipe;
    pub const WORKSPACE_ROOT_ENV: &str = "_WORKSPACE_ROOT";
}

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::ErrorKind::WouldBlock;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};
use std::{panic, thread};

use anyhow::{anyhow, Context, Result};
use log::{debug, warn};

use super::common::{Assistant, AssistantKind, Baselines, BenchmarkSummaries, Config, ModulePath};
use super::format::{BinaryBenchmarkHeader, OutputFormat};
use super::meta::Metadata;
use super::summary::{BaselineKind, BaselineName, BenchmarkKind, BenchmarkSummary, SummaryOutput};
use super::tool::config::ToolConfigs;
use super::tool::path::{ToolOutputPath, ToolOutputPathKind};
use super::tool::run::RunOptions;
use crate::api::{
    self, BinaryBenchmarkBench, BinaryBenchmarkConfig, BinaryBenchmarkGroups, DelayKind,
    EntryPoint, Stdin, ValgrindTool,
};
use crate::error::Error;
use crate::runner::format;

#[derive(Debug)]
struct BaselineBenchmark {
    baseline_kind: BaselineKind,
}

/// A `BinBench` represents a single benchmark under the `#[binary_benchmark]` macro
#[derive(Debug)]
pub struct BinBench {
    /// The arguments of `args` attribute as a single string
    pub args: Option<String>,
    /// The [`Command`] to execute under valgrind
    pub command: Command,
    /// The default [`ValgrindTool`]. If not changed it is `Callgrind`.
    pub default_tool: ValgrindTool,
    /// The name of the annotated function
    pub function_name: String,
    /// The id of the benchmark as in `#[bench::id]`
    pub id: Option<String>,
    /// The [`ModulePath`].
    pub module_path: ModulePath,
    /// The [`OutputFormat`]
    pub output_format: OutputFormat,
    /// The [`RunOptions`]
    pub run_options: RunOptions,
    /// The tool configurations for this benchmark run
    pub tools: ToolConfigs,
}

/// The Command derived from the `api::Command`
///
/// If the path is relative we convert it to an absolute path relative to the workspace root.
/// `stdin`, `stdout`, `stderr` of the `api::Command` are part of the `RunOptions` and not part of
/// this `Command`
#[derive(Debug, Clone)]
pub struct Command {
    /// The arguments to pass to the executable
    pub args: Vec<OsString>,
    /// The path to the executable
    pub path: PathBuf,
}

/// The `Delay` which should be applied to the [`Command`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delay {
    /// The kind of delay
    pub kind: DelayKind,
    /// The polling time to check the delay condition
    pub poll: Duration,
    /// The timeout for the delay
    pub timeout: Duration,
}

#[derive(Debug)]
struct Group {
    benches: Vec<BinBench>,
    compare_by_id: bool,
    /// The module path so far which should be `file_name::group_name`
    module_path: ModulePath,
    /// This name is the name from the `library_benchmark_group!` macro
    ///
    /// Due to the way we expand the `library_benchmark_group!` macro, we can safely assume that
    /// this name is unique.
    name: String,
    setup: Option<Assistant>,
    teardown: Option<Assistant>,
}

#[derive(Debug)]
struct Groups(Vec<Group>);

#[derive(Debug)]
struct LoadBaselineBenchmark {
    baseline: BaselineName,
    loaded_baseline: BaselineName,
}

#[derive(Debug)]
struct Runner {
    benchmark: Box<dyn Benchmark>,
    config: Config,
    groups: Groups,
    setup: Option<Assistant>,
    teardown: Option<Assistant>,
}

#[derive(Debug)]
struct SaveBaselineBenchmark {
    baseline: BaselineName,
}

trait Benchmark: std::fmt::Debug {
    fn baselines(&self) -> Baselines;
    fn output_path(&self, bin_bench: &BinBench, config: &Config, group: &Group) -> ToolOutputPath;
    fn run(&self, bin_bench: &BinBench, config: &Config, group: &Group)
        -> Result<BenchmarkSummary>;
}

impl Benchmark for BaselineBenchmark {
    fn output_path(&self, bin_bench: &BinBench, config: &Config, group: &Group) -> ToolOutputPath {
        let kind = if bin_bench.default_tool.has_output_file() {
            ToolOutputPathKind::Out
        } else {
            ToolOutputPathKind::Log
        };
        ToolOutputPath::new(
            kind,
            bin_bench.default_tool,
            &self.baseline_kind,
            &config.meta.target_dir,
            &group.module_path,
            &bin_bench.name(),
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
        bin_bench: &BinBench,
        config: &Config,
        group: &Group,
    ) -> Result<BenchmarkSummary> {
        let header = BinaryBenchmarkHeader::new(&config.meta, bin_bench);
        header.print();

        let out_path = self.output_path(bin_bench, config, group);
        out_path.init()?;

        for path in bin_bench.tools.output_paths(&out_path) {
            path.shift()?;
            if path.kind == ToolOutputPathKind::Out {
                path.to_log_output().shift()?;
            }
            if let Some(path) = path.to_xtree_output() {
                path.shift()?;
            }
            if let Some(path) = path.to_xleak_output() {
                path.shift()?;
            }
        }

        let benchmark_summary = bin_bench.create_benchmark_summary(
            config,
            &out_path,
            &bin_bench.function_name,
            header.description(),
            self.baselines(),
        )?;

        bin_bench.tools.run(
            &header.to_title(),
            benchmark_summary,
            &self.baselines(),
            &self.baseline_kind,
            config,
            &bin_bench.command.path,
            &bin_bench.command.args,
            &bin_bench.run_options,
            &out_path,
            false,
            &bin_bench.module_path,
            &bin_bench.output_format,
        )
    }
}

impl BinBench {
    #[allow(clippy::too_many_lines)]
    fn new(
        meta: &Metadata,
        group: &Group,
        config: BinaryBenchmarkConfig,
        group_index: usize,
        bench_index: usize,
        binary_benchmark_bench: BinaryBenchmarkBench,
        default_tool: ValgrindTool,
    ) -> Result<Self> {
        let module_path = group
            .module_path
            .join(&binary_benchmark_bench.function_name);

        let default_tool = meta
            .args
            .default_tool
            .unwrap_or_else(|| config.default_tool.unwrap_or(default_tool));

        let api::Command {
            path,
            args,
            stdin,
            stdout,
            stderr,
            delay,
            ..
        } = binary_benchmark_bench.command;

        let command = Command::new(&module_path, path, args).map_err(|error| {
            Error::ConfigurationError(
                module_path.clone(),
                binary_benchmark_bench.id.clone(),
                error.to_string(),
            )
        })?;

        let mut assistant_envs = config.collect_envs();
        assistant_envs.push((
            OsString::from(defaults::WORKSPACE_ROOT_ENV),
            meta.project_root.clone().into(),
        ));

        let command_envs = config.resolve_envs();

        let mut output_format = config
            .output_format
            .map_or_else(OutputFormat::default, Into::into);
        output_format.kind = meta.args.output_format;

        let tool_configs = ToolConfigs::new(
            &mut output_format,
            config.tools,
            &module_path,
            binary_benchmark_bench.id.as_ref(),
            meta,
            default_tool,
            &EntryPoint::None,
            &config.valgrind_args,
            &HashMap::default(),
        )
        .map_err(|error| {
            Error::ConfigurationError(
                module_path.clone(),
                binary_benchmark_bench.id.clone(),
                error.to_string(),
            )
        })?;

        let setup = binary_benchmark_bench
            .has_setup
            .then_some(Assistant::new_bench_assistant(
                AssistantKind::Setup,
                &group.name,
                (group_index, bench_index),
                stdin.as_ref().and_then(|s| {
                    if let Stdin::Setup(p) = s {
                        Some(*p)
                    } else {
                        None
                    }
                }),
                assistant_envs.clone(),
                config.setup_parallel.unwrap_or(false),
            ));
        let teardown =
            binary_benchmark_bench
                .has_teardown
                .then_some(Assistant::new_bench_assistant(
                    AssistantKind::Teardown,
                    &group.name,
                    (group_index, bench_index),
                    None,
                    assistant_envs,
                    false,
                ));

        Ok(Self {
            id: binary_benchmark_bench.id,
            args: binary_benchmark_bench.args,
            function_name: binary_benchmark_bench.function_name,
            tools: tool_configs,
            run_options: RunOptions {
                env_clear: config.env_clear.unwrap_or(defaults::ENV_CLEAR),
                envs: command_envs,
                stdin: stdin.or(Some(defaults::STDIN)),
                stdout,
                stderr,
                exit_with: config.exit_with,
                current_dir: config.current_dir,
                setup,
                teardown,
                sandbox: config.sandbox,
                delay: delay.map(Into::into),
            },
            module_path,
            command,
            output_format,
            default_tool,
        })
    }

    fn name(&self) -> String {
        if let Some(bench_id) = &self.id {
            format!("{}.{}", self.function_name, bench_id)
        } else {
            self.function_name.clone()
        }
    }

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
            BenchmarkKind::BinaryBenchmark,
            config.meta.project_root.clone(),
            config.package_dir.clone(),
            config.bench_file.clone(),
            self.command.path.clone(),
            &self.module_path,
            function_name,
            self.id.clone(),
            description,
            summary_output,
            baselines,
        ))
    }
}

impl Command {
    fn new(module_path: &ModulePath, path: PathBuf, args: Vec<OsString>) -> Result<Self> {
        if path.as_os_str().is_empty() {
            return Err(anyhow!("{module_path}: Empty path in command",));
        }

        Ok(Self { args, path })
    }
}

impl Delay {
    /// Create a new `Delay`
    pub fn new(poll: Duration, timeout: Duration, kind: DelayKind) -> Self {
        Self {
            kind,
            poll,
            timeout,
        }
    }

    /// Apply the `Delay`
    pub fn run(&self) -> Result<()> {
        if let DelayKind::DurationElapse(_) = self.kind {
            self.exec_delay_fn()
        } else {
            let (tx, rx) = mpsc::channel::<std::result::Result<(), anyhow::Error>>();

            let delay = self.clone();
            let handle = thread::spawn(move || {
                tx.send(delay.exec_delay_fn()).map_err(|error| {
                    anyhow!("Command::Delay MPSC channel send error. Error: {error:?}")
                })
            });

            match rx.recv_timeout(self.timeout) {
                Ok(result) => {
                    // These unwraps are safe
                    handle.join().unwrap().unwrap();
                    result.map(|()| debug!("Command::Delay successfully executed."))
                }
                Err(RecvTimeoutError::Timeout) => {
                    Err(anyhow!("Timeout of '{:?}' reached", self.timeout))
                }
                Err(RecvTimeoutError::Disconnected) => {
                    // The disconnect is caused by a panic in the thread, so the `unwrap_err` is
                    // safe. We propagate the panic as is.
                    panic::resume_unwind(handle.join().unwrap_err())
                }
            }
        }
    }

    fn exec_delay_fn(&self) -> Result<()> {
        match &self.kind {
            DelayKind::DurationElapse(duration) => {
                thread::sleep(*duration);
            }
            DelayKind::TcpConnect(addr) => {
                while let Err(_err) = TcpStream::connect(addr) {
                    thread::sleep(self.poll);
                }
            }
            DelayKind::UdpResponse(remote, req) => {
                let socket = match remote {
                    SocketAddr::V4(_) => {
                        UdpSocket::bind(SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), 0))
                            .context("Could not bind local IPv4 UDP socket.")?
                    }
                    SocketAddr::V6(_) => {
                        UdpSocket::bind(SocketAddr::new(Ipv6Addr::UNSPECIFIED.into(), 0))
                            .context("Could not bind local IPv6 UDP socket.")?
                    }
                };

                socket.set_read_timeout(Some(self.poll))?;
                socket.set_write_timeout(Some(self.poll))?;

                loop {
                    while let Err(_err) = socket.send_to(req.as_slice(), remote) {
                        thread::sleep(self.poll);
                    }

                    let mut buf = [0; 1];
                    match socket.recv(&mut buf) {
                        Ok(_size) => break,
                        Err(e) => {
                            if e.kind() != WouldBlock {
                                thread::sleep(self.poll);
                            }
                        }
                    }
                }
            }
            DelayKind::PathExists(path) => {
                let wait_for_path = std::path::PathBuf::from(Path::new(path));
                while !wait_for_path.exists() {
                    thread::sleep(self.poll);
                }
            }
        }

        Ok(())
    }
}

impl From<api::Delay> for Delay {
    fn from(value: api::Delay) -> Self {
        let (poll, timeout) = if let DelayKind::DurationElapse(_) = value.kind {
            if value.poll.is_some() {
                warn!("Ignoring poll setting. Not supported for {:?}", value.kind);
            }
            if value.timeout.is_some() {
                warn!(
                    "Ignoring timeout setting. Not supported for {:?}",
                    value.kind
                );
            }
            (Duration::ZERO, Duration::ZERO)
        } else {
            let mut poll = value.poll.unwrap_or_else(|| Duration::from_millis(10));
            let timeout = value.timeout.map_or_else(
                || Duration::from_secs(600),
                |t| {
                    if t < Duration::from_millis(10) {
                        warn!("The minimum timeout setting is 10ms");
                        Duration::from_millis(10)
                    } else {
                        t
                    }
                },
            );

            if poll >= timeout {
                warn!(
                    "Poll duration is equal to or greater than the timeout duration ({poll:?} >= \
                     {timeout:?})."
                );
                poll = timeout - Duration::from_millis(5);
                warn!("Using poll duration {poll:?} instead");
            }
            (poll, timeout)
        };

        Self {
            poll,
            timeout,
            kind: value.kind,
        }
    }
}

impl Group {
    fn run(&self, benchmark: &dyn Benchmark, config: &Config) -> Result<BenchmarkSummaries> {
        let mut benchmark_summaries = BenchmarkSummaries::default();

        let mut summaries: HashMap<String, Vec<BenchmarkSummary>> =
            HashMap::with_capacity(self.benches.len());
        for bench in &self.benches {
            let fail_fast = bench
                .tools
                .0
                .iter()
                .any(|c| c.regression_config.is_fail_fast());

            let summary = benchmark.run(bench, config, self)?;
            summary.print_and_save(&config.meta.args.output_format)?;
            summary.check_regression(fail_fast)?;

            benchmark_summaries.add_summary(summary.clone());
            if self.compare_by_id && bench.output_format.is_default() {
                if let Some(id) = &summary.id {
                    if let Some(sums) = summaries.get_mut(id) {
                        for sum in sums.iter() {
                            sum.compare_and_print(id, &summary, &bench.output_format)?;
                        }
                        sums.push(summary);
                    } else {
                        summaries.insert(id.clone(), vec![summary]);
                    }
                }
            }
        }

        Ok(benchmark_summaries)
    }
}

impl Groups {
    fn from_binary_benchmark(
        module: &ModulePath,
        benchmark_groups: BinaryBenchmarkGroups,
        meta: &Metadata,
    ) -> Result<Self> {
        let global_config = benchmark_groups.config;
        let default_tool = benchmark_groups.default_tool;

        let mut groups = vec![];
        for binary_benchmark_group in benchmark_groups.groups {
            let group_module_path = module.join(&binary_benchmark_group.id);
            let group_config = global_config
                .clone()
                .update_from_all([binary_benchmark_group.config.as_ref()]);

            let setup = binary_benchmark_group
                .has_setup
                .then_some(Assistant::new_group_assistant(
                    AssistantKind::Setup,
                    &binary_benchmark_group.id,
                    group_config.collect_envs(),
                    false,
                ));
            let teardown =
                binary_benchmark_group
                    .has_teardown
                    .then_some(Assistant::new_group_assistant(
                        AssistantKind::Teardown,
                        &binary_benchmark_group.id,
                        group_config.collect_envs(),
                        false,
                    ));

            let mut group = Group {
                name: binary_benchmark_group.id,
                module_path: group_module_path,
                benches: vec![],
                setup,
                teardown,
                compare_by_id: binary_benchmark_group
                    .compare_by_id
                    .unwrap_or(defaults::COMPARE_BY_ID),
            };

            for (group_index, binary_benchmark_benches) in binary_benchmark_group
                .binary_benchmarks
                .into_iter()
                .enumerate()
            {
                for (bench_index, binary_benchmark_bench) in
                    binary_benchmark_benches.benches.into_iter().enumerate()
                {
                    let config = group_config.clone().update_from_all([
                        binary_benchmark_benches.config.as_ref(),
                        binary_benchmark_bench.config.as_ref(),
                        Some(&binary_benchmark_bench.command.config),
                    ]);

                    let bin_bench = BinBench::new(
                        meta,
                        &group,
                        config,
                        group_index,
                        bench_index,
                        binary_benchmark_bench,
                        default_tool,
                    )?;
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
    fn run(&self, benchmark: &dyn Benchmark, config: &Config) -> Result<BenchmarkSummaries> {
        let mut benchmark_summaries = BenchmarkSummaries::default();
        for group in &self.0 {
            if let Some(setup) = &group.setup {
                setup.run(config, &group.module_path)?;
            }

            let summaries = group.run(benchmark, config)?;

            if let Some(teardown) = &group.teardown {
                teardown.run(config, &group.module_path)?;
            }

            benchmark_summaries.add_other(summaries);
        }

        Ok(benchmark_summaries)
    }
}

impl Benchmark for LoadBaselineBenchmark {
    fn output_path(&self, bin_bench: &BinBench, config: &Config, group: &Group) -> ToolOutputPath {
        let kind = if bin_bench.default_tool.has_output_file() {
            ToolOutputPathKind::BaseOut(self.loaded_baseline.to_string())
        } else {
            ToolOutputPathKind::BaseLog(self.loaded_baseline.to_string())
        };
        ToolOutputPath::new(
            kind,
            bin_bench.default_tool,
            &BaselineKind::Name(self.baseline.clone()),
            &config.meta.target_dir,
            &group.module_path,
            &bin_bench.name(),
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
        bin_bench: &BinBench,
        config: &Config,
        group: &Group,
    ) -> Result<BenchmarkSummary> {
        let header = BinaryBenchmarkHeader::new(&config.meta, bin_bench);
        header.print();

        let out_path = self.output_path(bin_bench, config, group);
        let benchmark_summary = bin_bench.create_benchmark_summary(
            config,
            &out_path,
            &bin_bench.function_name,
            header.description(),
            self.baselines(),
        )?;

        bin_bench.tools.run_loaded_vs_base(
            &header.to_title(),
            &self.baseline,
            &self.loaded_baseline,
            benchmark_summary,
            &self.baselines(),
            config,
            &out_path,
            &bin_bench.output_format,
        )
    }
}

impl Runner {
    fn new(benchmark_groups: BinaryBenchmarkGroups, config: Config) -> Result<Self> {
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
            Groups::from_binary_benchmark(&config.module_path, benchmark_groups, &config.meta)?;

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
            benchmark,
            config,
            groups,
            setup,
            teardown,
        })
    }

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
    fn output_path(&self, bin_bench: &BinBench, config: &Config, group: &Group) -> ToolOutputPath {
        let kind = if bin_bench.default_tool.has_output_file() {
            ToolOutputPathKind::BaseOut(self.baseline.to_string())
        } else {
            ToolOutputPathKind::BaseLog(self.baseline.to_string())
        };
        ToolOutputPath::new(
            kind,
            bin_bench.default_tool,
            &BaselineKind::Name(self.baseline.clone()),
            &config.meta.target_dir,
            &group.module_path,
            &bin_bench.name(),
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
        bin_bench: &BinBench,
        config: &Config,
        group: &Group,
    ) -> Result<BenchmarkSummary> {
        let header = BinaryBenchmarkHeader::new(&config.meta, bin_bench);
        header.print();

        let out_path = self.output_path(bin_bench, config, group);
        out_path.init()?;

        let benchmark_summary = bin_bench.create_benchmark_summary(
            config,
            &out_path,
            &bin_bench.function_name,
            header.description(),
            self.baselines(),
        )?;

        bin_bench.tools.run(
            &header.to_title(),
            benchmark_summary,
            &self.baselines(),
            &BaselineKind::Name(self.baseline.clone()),
            config,
            &bin_bench.command.path,
            &bin_bench.command.args,
            &bin_bench.run_options,
            &out_path,
            true,
            &bin_bench.module_path,
            &bin_bench.output_format,
        )
    }
}

/// Print a list of all benchmarks with a short summary
pub fn list(benchmark_groups: BinaryBenchmarkGroups, config: &Config) -> Result<()> {
    let groups =
        Groups::from_binary_benchmark(&config.module_path, benchmark_groups, &config.meta)?;

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

/// The top-level method which should be used to initiate running all benchmarks
pub fn run(benchmark_groups: BinaryBenchmarkGroups, config: Config) -> Result<BenchmarkSummaries> {
    let runner = Runner::new(benchmark_groups, config)?;

    let start = Instant::now();
    let mut summaries = runner.run()?;
    summaries.elapsed(start);

    Ok(summaries)
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::net::TcpListener;

    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use tempfile::tempdir;

    use super::*;

    fn api_delay_fixture<T, U>(poll: T, timeout: U, kind: DelayKind) -> api::Delay
    where
        T: Into<Option<u64>>,
        U: Into<Option<u64>>,
    {
        api::Delay {
            poll: poll.into().map(Duration::from_millis),
            timeout: timeout.into().map(Duration::from_millis),
            kind,
        }
    }

    #[rstest]
    #[case::duration_elapse_when_no_poll_no_timeout(
        api_delay_fixture(None, None, DelayKind::DurationElapse(Duration::from_millis(100))),
        Duration::ZERO,
        Duration::ZERO
    )]
    #[case::duration_elapse_when_poll_no_timeout(
        api_delay_fixture(10, None, DelayKind::DurationElapse(Duration::from_millis(100))),
        Duration::ZERO,
        Duration::ZERO
    )]
    #[case::duration_elapse_when_no_poll_but_timeout(
        api_delay_fixture(None, 10, DelayKind::DurationElapse(Duration::from_millis(100))),
        Duration::ZERO,
        Duration::ZERO
    )]
    #[case::duration_elapse_when_poll_and_timeout(
        api_delay_fixture(10, 100, DelayKind::DurationElapse(Duration::from_millis(100))),
        Duration::ZERO,
        Duration::ZERO
    )]
    #[case::path_when_no_poll_no_timeout(
        api_delay_fixture(None, None, DelayKind::PathExists(PathBuf::from("/some/path"))),
        Duration::from_millis(10),
        Duration::from_secs(600)
    )]
    #[case::path_when_poll_no_timeout(
        api_delay_fixture(20, None, DelayKind::PathExists(PathBuf::from("/some/path"))),
        Duration::from_millis(20),
        Duration::from_secs(600)
    )]
    #[case::path_when_no_poll_but_timeout(
        api_delay_fixture(None, 200, DelayKind::PathExists(PathBuf::from("/some/path"))),
        Duration::from_millis(10),
        Duration::from_millis(200)
    )]
    #[case::path_when_poll_and_timeout(
        api_delay_fixture(20, 200, DelayKind::PathExists(PathBuf::from("/some/path"))),
        Duration::from_millis(20),
        Duration::from_millis(200)
    )]
    #[case::path_when_poll_equal_to_timeout(
        api_delay_fixture(200, 200, DelayKind::PathExists(PathBuf::from("/some/path"))),
        Duration::from_millis(195),
        Duration::from_millis(200)
    )]
    #[case::path_when_poll_higher_than_timeout(
        api_delay_fixture(201, 200, DelayKind::PathExists(PathBuf::from("/some/path"))),
        Duration::from_millis(195),
        Duration::from_millis(200)
    )]
    #[case::path_when_poll_equal_to_timeout_smaller_than_10(
        api_delay_fixture(10, 9, DelayKind::PathExists(PathBuf::from("/some/path"))),
        Duration::from_millis(5),
        Duration::from_millis(10)
    )]
    #[case::path_when_poll_lower_than_timeout_smaller_than_10(
        api_delay_fixture(7, 9, DelayKind::PathExists(PathBuf::from("/some/path"))),
        Duration::from_millis(7),
        Duration::from_millis(10)
    )]
    fn test_from_api_delay_for_delay(
        #[case] delay: api::Delay,
        #[case] poll: Duration,
        #[case] timeout: Duration,
    ) {
        let expected = Delay::new(poll, timeout, delay.kind.clone());
        assert_eq!(Delay::from(delay), expected);
    }

    #[test]
    fn test_delay_path() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("file.pid");

        let delay = Delay {
            poll: Duration::from_millis(50),
            timeout: Duration::from_millis(200),
            kind: DelayKind::PathExists(file_path.clone()),
        };
        let handle = thread::spawn(move || {
            delay.run().unwrap();
        });

        thread::sleep(Duration::from_millis(100));
        File::create(file_path).unwrap();

        handle.join().unwrap();
        drop(dir);
    }

    #[test]
    fn test_delay_tcp_connect() {
        let addr = "127.0.0.1:32000".parse::<SocketAddr>().unwrap();
        let _listener = TcpListener::bind(addr).unwrap();

        let delay = Delay {
            poll: Duration::from_millis(20),
            timeout: Duration::from_secs(1),
            kind: DelayKind::TcpConnect(addr),
        };
        delay.run().unwrap();
    }

    #[test]
    fn test_delay_tcp_connect_poll() {
        let addr = "127.0.0.1:32001".parse::<SocketAddr>().unwrap();

        let check_addr = addr;
        let handle = thread::spawn(move || {
            let delay = Delay {
                poll: Duration::from_millis(20),
                timeout: Duration::from_secs(1),
                kind: DelayKind::TcpConnect(check_addr),
            };
            delay.run().unwrap();
        });

        thread::sleep(Duration::from_millis(100));
        let _listener = TcpListener::bind(addr).unwrap();

        handle.join().unwrap();
    }

    #[test]
    fn test_delay_tcp_connect_timeout() {
        let addr = "127.0.0.1:32002".parse::<SocketAddr>().unwrap();
        let delay = Delay {
            poll: Duration::from_millis(20),
            timeout: Duration::from_secs(1),
            kind: DelayKind::TcpConnect(addr),
        };

        let result = delay.run();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Timeout of '1s' reached");
    }

    #[test]
    fn test_delay_udp_response() {
        let addr = "127.0.0.1:34000".parse::<SocketAddr>().unwrap();

        thread::spawn(move || -> ! {
            let server = UdpSocket::bind(addr).unwrap();
            server
                .set_read_timeout(Some(Duration::from_millis(100)))
                .unwrap();
            server
                .set_write_timeout(Some(Duration::from_millis(100)))
                .unwrap();

            loop {
                let mut buf = [0; 1];

                match server.recv_from(&mut buf) {
                    Ok((_size, from)) => {
                        server.send_to(&[2], from).unwrap();
                    }
                    Err(_e) => {}
                }
            }
        });

        let delay = Delay {
            poll: Duration::from_millis(20),
            timeout: Duration::from_millis(100),
            kind: DelayKind::UdpResponse(addr, vec![1]),
        };

        delay.run().unwrap();
    }

    #[test]
    fn test_delay_udp_response_poll() {
        let addr = "127.0.0.1:34001".parse::<SocketAddr>().unwrap();

        thread::spawn(move || {
            let delay = Delay {
                poll: Duration::from_millis(20),
                timeout: Duration::from_millis(100),
                kind: DelayKind::UdpResponse(addr, vec![1]),
            };
            delay.run().unwrap();
        });

        let server = UdpSocket::bind(addr).unwrap();
        server
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        server
            .set_write_timeout(Some(Duration::from_millis(100)))
            .unwrap();

        loop {
            let mut buf = [0; 1];

            thread::sleep(Duration::from_millis(70));

            match server.recv_from(&mut buf) {
                Ok((_size, from)) => {
                    server.send_to(&[2], from).unwrap();
                    break;
                }
                Err(_e) => {}
            }
        }
    }

    #[test]
    fn test_delay_udp_response_timeout() {
        let addr = "127.0.0.1:34002".parse::<SocketAddr>().unwrap();
        let delay = Delay {
            poll: Duration::from_millis(20),
            timeout: Duration::from_millis(100),
            kind: DelayKind::UdpResponse(addr, vec![1]),
        };
        let result = delay.run();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Timeout of '100ms' reached"
        );
    }
}
