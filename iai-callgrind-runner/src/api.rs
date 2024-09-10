//! The api contains all elements which the `runner` can understand
use std::ffi::OsString;
use std::fmt::Display;
#[cfg(feature = "runner")]
use std::fs::File;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
#[cfg(feature = "runner")]
use std::process::{Child, Command as StdCommand, Stdio as StdStdio};
use std::time::Duration;

#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::api::DelayKind::DurationElapse;

/// The model for the `#[binary_benchmark]` attribute or the equivalent from the low level api
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BinaryBenchmark {
    pub config: Option<BinaryBenchmarkConfig>,
    pub benches: Vec<BinaryBenchmarkBench>,
}

/// The model for the `#[bench]` attribute or the low level equivalent
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BinaryBenchmarkBench {
    pub id: Option<String>,
    pub function_name: String,
    pub args: Option<String>,
    pub command: Command,
    pub config: Option<BinaryBenchmarkConfig>,
    pub has_setup: bool,
    pub has_teardown: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BinaryBenchmarkConfig {
    pub env_clear: Option<bool>,
    pub current_dir: Option<PathBuf>,
    pub entry_point: Option<String>,
    pub exit_with: Option<ExitWith>,
    pub raw_callgrind_args: RawArgs,
    pub envs: Vec<(OsString, Option<OsString>)>,
    pub flamegraph_config: Option<FlamegraphConfig>,
    pub regression_config: Option<RegressionConfig>,
    pub tools: Tools,
    pub tools_override: Option<Tools>,
    pub sandbox: Option<Sandbox>,
    pub truncate_description: Option<Option<usize>>,
    pub setup_parallel: Option<bool>,
}

/// The model for the `binary_benchmark_group` macro
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BinaryBenchmarkGroup {
    pub id: String,
    pub config: Option<BinaryBenchmarkConfig>,
    pub has_setup: bool,
    pub has_teardown: bool,
    pub binary_benchmarks: Vec<BinaryBenchmark>,
    pub compare_by_id: Option<bool>,
}

/// The model for the main! macro
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BinaryBenchmarkGroups {
    pub config: BinaryBenchmarkConfig,
    pub groups: Vec<BinaryBenchmarkGroup>,
    /// The command line arguments as we receive them from `cargo bench`
    pub command_line_args: Vec<String>,
    pub has_setup: bool,
    pub has_teardown: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Delay {
    pub poll: Option<Duration>,
    pub timeout: Option<Duration>,
    pub kind: DelayKind,
}

/// The kind of `Delay`
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelayKind {
    /// Delay the `Command` for a fixed [`Duration`]
    DurationElapse(Duration),
    /// Delay the `Command` until a successful tcp connection can be established
    TcpConnect(SocketAddr),
    /// Delay the `Command` until a successful udp response was received
    UdpResponse(SocketAddr, Vec<u8>),
    /// Delay the `Command` until the specified path exists
    PathExists(PathBuf),
}

impl Default for DelayKind {
    fn default() -> Self {
        DurationElapse(Duration::from_secs(60))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Command {
    pub path: PathBuf,
    pub args: Vec<OsString>,
    pub stdin: Option<Stdin>,
    pub stdout: Option<Stdio>,
    pub stderr: Option<Stdio>,
    pub config: BinaryBenchmarkConfig,
    pub delay: Option<Delay>,
}

/// The `Direction` in which the flamegraph should grow.
///
/// The default is `TopToBottom`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    /// Grow from top to bottom with the highest event costs at the top
    TopToBottom,
    /// Grow from bottom to top with the highest event costs at the bottom
    BottomToTop,
}

/// The `EntryPoint` of a library benchmark
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum EntryPoint {
    /// Disable the entry point and default toggle entirely
    None,
    /// The default entry point is the benchmark function
    #[default]
    Default,
    /// A custom entry point. The argument allows the same glob patterns as the
    /// [`--toggle-collect`](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options)
    /// argument of callgrind.
    Custom(String),
}

/// All `EventKind`s callgrind produces and additionally some derived events
///
/// Depending on the options passed to Callgrind, these are the events that Callgrind can produce.
/// See the [Callgrind
/// documentation](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options) for details.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum EventKind {
    /// The default event. I cache reads (which equals the number of instructions executed)
    Ir,
    /// The number of system calls done (--collect-systime=yes)
    SysCount,
    /// The elapsed time spent in system calls (--collect-systime=yes)
    SysTime,
    /// The cpu time spent during system calls (--collect-systime=nsec)
    SysCpuTime,
    /// The number of global bus events (--collect-bus=yes)
    Ge,
    /// D Cache reads (which equals the number of memory reads) (--cache-sim=yes)
    Dr,
    /// D Cache writes (which equals the number of memory writes) (--cache-sim=yes)
    Dw,
    /// I1 cache read misses (--cache-sim=yes)
    I1mr,
    /// LL cache instruction read misses (--cache-sim=yes)
    ILmr,
    /// D1 cache read misses (--cache-sim=yes)
    D1mr,
    /// LL cache data read misses (--cache-sim=yes)
    DLmr,
    /// D1 cache write misses (--cache-sim=yes)
    D1mw,
    /// LL cache data write misses (--cache-sim=yes)
    DLmw,
    /// Derived event showing the L1 hits (--cache-sim=yes)
    L1hits,
    /// Derived event showing the LL hits (--cache-sim=yes)
    LLhits,
    /// Derived event showing the RAM hits (--cache-sim=yes)
    RamHits,
    /// Derived event showing the total amount of cache reads and writes (--cache-sim=yes)
    TotalRW,
    /// Derived event showing estimated CPU cycles (--cache-sim=yes)
    EstimatedCycles,
    /// Conditional branches executed (--branch-sim=yes)
    Bc,
    /// Conditional branches mispredicted (--branch-sim=yes)
    Bcm,
    /// Indirect branches executed (--branch-sim=yes)
    Bi,
    /// Indirect branches mispredicted (--branch-sim=yes)
    Bim,
    /// Dirty miss because of instruction read (--simulate-wb=yes)
    ILdmr,
    /// Dirty miss because of data read (--simulate-wb=yes)
    DLdmr,
    /// Dirty miss because of data write (--simulate-wb=yes)
    DLdmw,
    /// Counter showing bad temporal locality for L1 caches (--cachuse=yes)
    AcCost1,
    /// Counter showing bad temporal locality for LL caches (--cachuse=yes)
    AcCost2,
    /// Counter showing bad spatial locality for L1 caches (--cachuse=yes)
    SpLoss1,
    /// Counter showing bad spatial locality for LL caches (--cachuse=yes)
    SpLoss2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitWith {
    Success,
    Failure,
    Code(i32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixtures {
    pub path: PathBuf,
    pub follow_symlinks: bool,
}

/// TODO: ADD OPTION to create flamegraphs for all files not just the total
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FlamegraphConfig {
    pub kind: Option<FlamegraphKind>,
    pub negate_differential: Option<bool>,
    pub normalize_differential: Option<bool>,
    pub event_kinds: Option<Vec<EventKind>>,
    pub direction: Option<Direction>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub min_width: Option<f64>,
}

/// The kind of `Flamegraph` which is going to be constructed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlamegraphKind {
    /// The regular flamegraph for the new callgrind run
    Regular,
    /// A differential flamegraph showing the differences between the new and old callgrind run
    Differential,
    /// All flamegraph kinds that can be constructed (`Regular` and `Differential`). This
    /// is the default.
    All,
    /// Do not produce any flamegraphs
    None,
}

/// The model for the `#[library_benchmark]` attribute
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LibraryBenchmark {
    pub config: Option<LibraryBenchmarkConfig>,
    pub benches: Vec<LibraryBenchmarkBench>,
}

/// The model for the `#[bench]` attribute in a `#[library_benchmark]`
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LibraryBenchmarkBench {
    pub id: Option<String>,
    pub function_name: String,
    pub args: Option<String>,
    pub config: Option<LibraryBenchmarkConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LibraryBenchmarkConfig {
    pub env_clear: Option<bool>,
    pub raw_callgrind_args: RawArgs,
    pub envs: Vec<(OsString, Option<OsString>)>,
    pub flamegraph_config: Option<FlamegraphConfig>,
    pub regression_config: Option<RegressionConfig>,
    pub tools: Tools,
    pub tools_override: Option<Tools>,
    pub truncate_description: Option<Option<usize>>,
    pub entry_point: Option<EntryPoint>,
}

/// The model for the `library_benchmark_group` macro
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LibraryBenchmarkGroup {
    pub id: String,
    pub config: Option<LibraryBenchmarkConfig>,
    pub compare_by_id: Option<bool>,
    pub library_benchmarks: Vec<LibraryBenchmark>,
    pub has_setup: bool,
    pub has_teardown: bool,
}

/// The model for the `main` macro
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LibraryBenchmarkGroups {
    pub config: LibraryBenchmarkConfig,
    pub groups: Vec<LibraryBenchmarkGroup>,
    /// The command line args as we receive them from `cargo bench`
    pub command_line_args: Vec<String>,
    pub has_setup: bool,
    pub has_teardown: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawArgs(pub Vec<String>);

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RegressionConfig {
    pub limits: Vec<(EventKind, f64)>,
    pub fail_fast: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sandbox {
    pub enabled: Option<bool>,
    pub fixtures: Vec<PathBuf>,
    pub follow_symlinks: Option<bool>,
}

/// Configure the `Stream` which should be used as pipe in [`Stdin::Setup`]
///
/// The default is [`Pipe::Stdout`]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pipe {
    /// The `Stdout` default `Stream`
    #[default]
    Stdout,
    /// The `Stderr` error `Stream`
    Stderr,
}

/// We use this enum only internally in the benchmark runner
#[cfg(feature = "runner")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stream {
    Stdin,
    Stdout,
    Stderr,
}

/// Configure the `Stdio` of `Stdin`, `Stdout` and `Stderr`
///
/// Describes what to do with a standard I/O stream for the [`Command`] when passed to the stdin,
/// stdout, and stderr methods of [`Command`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stdio {
    /// The [`Command`]'s `Stream` inherits from the benchmark runner.
    #[default]
    Inherit,
    /// This stream will be ignored. This is the equivalent of attaching the stream to `/dev/null`
    Null,
    /// Redirect the content of a file into this `Stream`. This is equivalent to a redirection in a
    /// shell for example for the `Stdout` of `my-command`: `my-command > some_file`
    File(PathBuf),
    /// A new pipe should be arranged to connect the benchmark runner and the [`Command`]
    Pipe,
}

/// This is a special `Stdio` for the stdin method of [`Command`]
///
/// Contains all the standard [`Stdio`] options and the [`Stdin::Setup`] option
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stdin {
    /// Using this in [`Command::stdin`] pipes the stream specified with [`Pipe`] of the `setup`
    /// function into the `Stdin` of the [`Command`]. In this case the `setup` and [`Command`] are
    /// executed in parallel instead of sequentially. See [`Command::stdin`] for more details.
    Setup(Pipe),
    #[default]
    /// See [`Stdio::Inherit`]
    Inherit,
    /// See [`Stdio::Null`]
    Null,
    /// See [`Stdio::File`]
    File(PathBuf),
    /// See [`Stdio::Pipe`]
    Pipe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tool {
    pub kind: ValgrindTool,
    pub enable: Option<bool>,
    pub raw_args: RawArgs,
    // TODO: Remove support for modifier ?? Or, Set pid modifier automatically and only support
    // environment variable modifier
    pub outfile_modifier: Option<String>,
    pub show_log: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tools(pub Vec<Tool>);

/// An enum with all possible valgrind tools
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValgrindTool {
    /// [Memcheck: a memory error detector](https://valgrind.org/docs/manual/mc-manual.html)
    Memcheck,
    /// [Helgrind: a thread error detector](https://valgrind.org/docs/manual/hg-manual.html)
    Helgrind,
    /// [DRD: a thread error detector](https://valgrind.org/docs/manual/drd-manual.html)
    DRD,
    /// [Massif: a heap profiler](https://valgrind.org/docs/manual/ms-manual.html)
    Massif,
    /// [DHAT: a dynamic heap analysis tool](https://valgrind.org/docs/manual/dh-manual.html)
    DHAT,
    /// [BBV: an experimental basic block vector generation tool](https://valgrind.org/docs/manual/bbv-manual.html)
    BBV,
}

impl BinaryBenchmarkConfig {
    pub fn update_from_all<'a, T>(mut self, others: T) -> Self
    where
        T: IntoIterator<Item = Option<&'a Self>>,
    {
        for other in others.into_iter().flatten() {
            self.env_clear = update_option(&self.env_clear, &other.env_clear);
            self.current_dir = update_option(&self.current_dir, &other.current_dir);
            self.entry_point = update_option(&self.entry_point, &other.entry_point);
            self.exit_with = update_option(&self.exit_with, &other.exit_with);

            self.raw_callgrind_args
                .extend_ignore_flag(other.raw_callgrind_args.0.iter());

            self.envs.extend_from_slice(&other.envs);
            self.flamegraph_config =
                update_option(&self.flamegraph_config, &other.flamegraph_config);
            self.regression_config =
                update_option(&self.regression_config, &other.regression_config);
            if let Some(other_tools) = &other.tools_override {
                self.tools = other_tools.clone();
            } else if !other.tools.is_empty() {
                self.tools.update_from_other(&other.tools);
            } else {
                // do nothing
            }

            self.sandbox = update_option(&self.sandbox, &other.sandbox);
            self.truncate_description =
                update_option(&self.truncate_description, &other.truncate_description);
            self.setup_parallel = update_option(&self.setup_parallel, &other.setup_parallel);
        }
        self
    }

    pub fn resolve_envs(&self) -> Vec<(OsString, OsString)> {
        self.envs
            .iter()
            .filter_map(|(key, value)| match value {
                Some(value) => Some((key.clone(), value.clone())),
                None => std::env::var_os(key).map(|value| (key.clone(), value)),
            })
            .collect()
    }

    pub fn collect_envs(&self) -> Vec<(OsString, OsString)> {
        self.envs
            .iter()
            .filter_map(|(key, option)| option.as_ref().map(|value| (key.clone(), value.clone())))
            .collect()
    }
}

impl Default for Direction {
    fn default() -> Self {
        Self::BottomToTop
    }
}

impl<T> From<T> for EntryPoint
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        EntryPoint::Custom(value.into())
    }
}

impl EventKind {
    /// Return true if this `EventKind` is a derived event
    ///
    /// Derived events are calculated from Callgrind's native event types. See also
    /// [`crate::runner::callgrind::model::Costs::make_summary`]. Currently all derived events are:
    ///
    /// * [`EventKind::L1hits`]
    /// * [`EventKind::LLhits`]
    /// * [`EventKind::RamHits`]
    /// * [`EventKind::TotalRW`]
    /// * [`EventKind::EstimatedCycles`]
    pub fn is_derived(&self) -> bool {
        matches!(
            self,
            EventKind::L1hits
                | EventKind::LLhits
                | EventKind::RamHits
                | EventKind::TotalRW
                | EventKind::EstimatedCycles
        )
    }

    pub fn from_str_ignore_case(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "ir" => Some(Self::Ir),
            "dr" => Some(Self::Dr),
            "dw" => Some(Self::Dw),
            "i1mr" => Some(Self::I1mr),
            "ilmr" => Some(Self::ILmr),
            "d1mr" => Some(Self::D1mr),
            "dlmr" => Some(Self::DLmr),
            "d1mw" => Some(Self::D1mw),
            "dlmw" => Some(Self::DLmw),
            "syscount" => Some(Self::SysCount),
            "systime" => Some(Self::SysTime),
            "syscputime" => Some(Self::SysCpuTime),
            "ge" => Some(Self::Ge),
            "bc" => Some(Self::Bc),
            "bcm" => Some(Self::Bcm),
            "bi" => Some(Self::Bi),
            "bim" => Some(Self::Bim),
            "ildmr" => Some(Self::ILdmr),
            "dldmr" => Some(Self::DLdmr),
            "dldmw" => Some(Self::DLdmw),
            "accost1" => Some(Self::AcCost1),
            "accost2" => Some(Self::AcCost2),
            "sploss1" => Some(Self::SpLoss1),
            "sploss2" => Some(Self::SpLoss2),
            "l1hits" => Some(Self::L1hits),
            "llhits" => Some(Self::LLhits),
            "ramhits" => Some(Self::RamHits),
            "totalrw" => Some(Self::TotalRW),
            "estimatedcycles" => Some(Self::EstimatedCycles),
            _ => None,
        }
    }

    pub fn to_name(&self) -> String {
        format!("{:?}", *self)
    }
}

impl Display for EventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventKind::Ir => f.write_str("Instructions"),
            EventKind::L1hits => f.write_str("L1 Hits"),
            EventKind::LLhits => f.write_str("L2 Hits"),
            EventKind::RamHits => f.write_str("RAM Hits"),
            EventKind::TotalRW => f.write_str("Total read+write"),
            EventKind::EstimatedCycles => f.write_str("Estimated Cycles"),
            _ => f.write_fmt(format_args!("{self:?}")),
        }
    }
}

impl<T> From<T> for EventKind
where
    T: AsRef<str>,
{
    fn from(value: T) -> Self {
        match value.as_ref() {
            "Ir" => Self::Ir,
            "Dr" => Self::Dr,
            "Dw" => Self::Dw,
            "I1mr" => Self::I1mr,
            "ILmr" => Self::ILmr,
            "D1mr" => Self::D1mr,
            "DLmr" => Self::DLmr,
            "D1mw" => Self::D1mw,
            "DLmw" => Self::DLmw,
            "sysCount" => Self::SysCount,
            "sysTime" => Self::SysTime,
            "sysCpuTime" => Self::SysCpuTime,
            "Ge" => Self::Ge,
            "Bc" => Self::Bc,
            "Bcm" => Self::Bcm,
            "Bi" => Self::Bi,
            "Bim" => Self::Bim,
            "ILdmr" => Self::ILdmr,
            "DLdmr" => Self::DLdmr,
            "DLdmw" => Self::DLdmw,
            "AcCost1" => Self::AcCost1,
            "AcCost2" => Self::AcCost2,
            "SpLoss1" => Self::SpLoss1,
            "SpLoss2" => Self::SpLoss2,
            "L1hits" => Self::L1hits,
            "LLhits" => Self::LLhits,
            "RamHits" => Self::RamHits,
            "TotalRW" => Self::TotalRW,
            "EstimatedCycles" => Self::EstimatedCycles,
            unknown => panic!("Unknown event type: {unknown}"),
        }
    }
}

impl LibraryBenchmarkConfig {
    pub fn update_from_all<'a, T>(mut self, others: T) -> Self
    where
        T: IntoIterator<Item = Option<&'a Self>>,
    {
        for other in others.into_iter().flatten() {
            self.env_clear = update_option(&self.env_clear, &other.env_clear);
            self.raw_callgrind_args
                .extend_ignore_flag(other.raw_callgrind_args.0.iter());
            self.envs.extend_from_slice(&other.envs);
            self.flamegraph_config =
                update_option(&self.flamegraph_config, &other.flamegraph_config);
            self.regression_config =
                update_option(&self.regression_config, &other.regression_config);
            if let Some(other_tools) = &other.tools_override {
                self.tools = other_tools.clone();
            } else if !other.tools.is_empty() {
                self.tools.update_from_other(&other.tools);
            } else {
                // do nothing
            }

            self.truncate_description =
                update_option(&self.truncate_description, &other.truncate_description);
            self.entry_point = update_option(&self.entry_point, &other.entry_point);
        }
        self
    }

    pub fn resolve_envs(&self) -> Vec<(OsString, OsString)> {
        self.envs
            .iter()
            .filter_map(|(key, value)| match value {
                Some(value) => Some((key.clone(), value.clone())),
                None => std::env::var_os(key).map(|value| (key.clone(), value)),
            })
            .collect()
    }

    pub fn collect_envs(&self) -> Vec<(OsString, OsString)> {
        self.envs
            .iter()
            .filter_map(|(key, option)| option.as_ref().map(|value| (key.clone(), value.clone())))
            .collect()
    }
}

impl RawArgs {
    pub fn new(args: Vec<String>) -> Self {
        Self(args)
    }

    pub fn extend_ignore_flag<I, T>(&mut self, args: T)
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.0.extend(
            args.into_iter()
                .filter(|s| !s.as_ref().is_empty())
                .map(|s| {
                    let string = s.as_ref();
                    if string.starts_with('-') {
                        string.to_owned()
                    } else {
                        format!("--{string}")
                    }
                }),
        );
    }

    pub fn from_command_line_args(args: Vec<String>) -> Self {
        let mut this = Self(Vec::default());
        if !args.is_empty() {
            let mut iter = args.into_iter();
            // This unwrap is safe. We just checked that `args` is not empty.
            let mut last = iter.next().unwrap();
            for elem in iter {
                this.0.push(last);
                last = elem;
            }
            if last.as_str() != "--bench" {
                this.0.push(last);
            }
        }
        this
    }
}

impl<I> FromIterator<I> for RawArgs
where
    I: AsRef<str>,
{
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        let mut this = Self::default();
        this.extend_ignore_flag(iter);
        this
    }
}

impl Stdin {
    #[cfg(feature = "runner")]
    pub(crate) fn apply(
        &self,
        command: &mut StdCommand,
        stream: Stream,
        child: Option<&mut Child>,
    ) -> Result<(), String> {
        match (self, child) {
            (Self::Setup(Pipe::Stdout), Some(child)) => {
                command.stdin(
                    child
                        .stdout
                        .take()
                        .ok_or_else(|| "Error piping setup stdout".to_owned())?,
                );
                Ok(())
            }
            (Self::Setup(Pipe::Stderr), Some(child)) => {
                command.stdin(
                    child
                        .stderr
                        .take()
                        .ok_or_else(|| "Error piping setup stderr".to_owned())?,
                );
                Ok(())
            }
            (Self::Setup(_) | Stdin::Pipe, _) => Stdio::Pipe.apply(command, stream),
            (Self::Inherit, _) => Stdio::Inherit.apply(command, stream),
            (Self::Null, _) => Stdio::Null.apply(command, stream),
            (Self::File(path), _) => Stdio::File(path.clone()).apply(command, stream),
        }
    }
}

impl From<Stdio> for Stdin {
    fn from(value: Stdio) -> Self {
        match value {
            Stdio::Inherit => Stdin::Inherit,
            Stdio::Null => Stdin::Null,
            Stdio::File(file) => Stdin::File(file),
            Stdio::Pipe => Stdin::Pipe,
        }
    }
}

impl From<PathBuf> for Stdin {
    fn from(value: PathBuf) -> Self {
        Self::File(value)
    }
}

impl From<&PathBuf> for Stdin {
    fn from(value: &PathBuf) -> Self {
        Self::File(value.to_owned())
    }
}

impl From<&Path> for Stdin {
    fn from(value: &Path) -> Self {
        Self::File(value.to_path_buf())
    }
}

impl Stdio {
    #[cfg(feature = "runner")]
    pub(crate) fn apply(&self, command: &mut StdCommand, stream: Stream) -> Result<(), String> {
        let stdio = match self {
            Stdio::Pipe => StdStdio::piped(),
            Stdio::Inherit => StdStdio::inherit(),
            Stdio::Null => StdStdio::null(),
            Stdio::File(path) => match stream {
                Stream::Stdin => StdStdio::from(File::open(path).map_err(|error| {
                    format!(
                        "Failed to open file '{}' in read mode for {stream}: {error}",
                        path.display()
                    )
                })?),
                Stream::Stdout | Stream::Stderr => {
                    StdStdio::from(File::create(path).map_err(|error| {
                        format!(
                            "Failed to create file '{}' for {stream}: {error}",
                            path.display()
                        )
                    })?)
                }
            },
        };

        match stream {
            Stream::Stdin => command.stdin(stdio),
            Stream::Stdout => command.stdout(stdio),
            Stream::Stderr => command.stderr(stdio),
        };

        Ok(())
    }

    #[cfg(feature = "runner")]
    pub(crate) fn is_pipe(&self) -> bool {
        match self {
            Stdio::Inherit => false,
            Stdio::Null | Stdio::File(_) | Stdio::Pipe => true,
        }
    }
}

impl From<PathBuf> for Stdio {
    fn from(value: PathBuf) -> Self {
        Self::File(value)
    }
}

impl From<&PathBuf> for Stdio {
    fn from(value: &PathBuf) -> Self {
        Self::File(value.to_owned())
    }
}

impl From<&Path> for Stdio {
    fn from(value: &Path) -> Self {
        Self::File(value.to_path_buf())
    }
}

#[cfg(feature = "runner")]
impl Display for Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{self:?}").to_lowercase())
    }
}

impl Tools {
    /// Return true if `Tools` is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Update `Tools`
    ///
    /// Adds the [`Tool`] to `Tools`. If a [`Tool`] is already present, it will be removed.
    ///
    /// This method is inefficient (computes in worst case O(n^2)) but since `Tools` has a
    /// manageable size with a maximum of 6 members we can spare us an `IndexSet` or similar and the
    /// dependency on it in `iai-callgrind`.
    pub fn update(&mut self, tool: Tool) {
        if let Some(pos) = self.0.iter().position(|t| t.kind == tool.kind) {
            self.0.remove(pos);
        }
        self.0.push(tool);
    }

    /// Update `Tools` with all [`Tool`]s from an iterator
    pub fn update_all<T>(&mut self, tools: T)
    where
        T: IntoIterator<Item = Tool>,
    {
        for tool in tools {
            self.update(tool);
        }
    }

    /// Update `Tools` with another `Tools`
    pub fn update_from_other(&mut self, tools: &Tools) {
        self.update_all(tools.0.iter().cloned());
    }
}

pub fn update_option<T: Clone>(first: &Option<T>, other: &Option<T>) -> Option<T> {
    other.clone().or_else(|| first.clone())
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_library_benchmark_config_update_from_all_when_default() {
        assert_eq!(
            LibraryBenchmarkConfig::default()
                .update_from_all([Some(&LibraryBenchmarkConfig::default())]),
            LibraryBenchmarkConfig::default()
        );
    }

    #[test]
    fn test_library_benchmark_config_update_from_all_when_no_tools_override() {
        let base = LibraryBenchmarkConfig::default();
        let other = LibraryBenchmarkConfig {
            env_clear: Some(true),
            raw_callgrind_args: RawArgs(vec!["--just-testing=yes".to_owned()]),
            envs: vec![(OsString::from("MY_ENV"), Some(OsString::from("value")))],
            flamegraph_config: Some(FlamegraphConfig::default()),
            regression_config: Some(RegressionConfig::default()),
            tools: Tools(vec![Tool {
                kind: ValgrindTool::DHAT,
                enable: None,
                raw_args: RawArgs(vec![]),
                outfile_modifier: None,
                show_log: None,
            }]),
            tools_override: None,
            truncate_description: None,
            entry_point: None,
        };

        assert_eq!(base.update_from_all([Some(&other.clone())]), other);
    }

    #[test]
    fn test_library_benchmark_config_update_from_all_when_tools_override() {
        let base = LibraryBenchmarkConfig::default();
        let other = LibraryBenchmarkConfig {
            env_clear: Some(true),
            raw_callgrind_args: RawArgs(vec!["--just-testing=yes".to_owned()]),
            envs: vec![(OsString::from("MY_ENV"), Some(OsString::from("value")))],
            flamegraph_config: Some(FlamegraphConfig::default()),
            regression_config: Some(RegressionConfig::default()),
            tools: Tools(vec![Tool {
                kind: ValgrindTool::DHAT,
                enable: None,
                raw_args: RawArgs(vec![]),
                outfile_modifier: None,
                show_log: None,
            }]),
            tools_override: Some(Tools(vec![])),
            truncate_description: Some(Some(10)),
            entry_point: Some(EntryPoint::default()),
        };
        let expected = LibraryBenchmarkConfig {
            tools: other.tools_override.as_ref().unwrap().clone(),
            tools_override: None,
            ..other.clone()
        };

        assert_eq!(base.update_from_all([Some(&other)]), expected);
    }

    #[rstest]
    #[case::truncate_description(
        LibraryBenchmarkConfig {
            truncate_description: Some(None),
            ..Default::default()
        }
    )]
    #[case::env_clear(
        LibraryBenchmarkConfig {
            env_clear: Some(true),
            ..Default::default()
        }
    )]
    fn test_library_benchmark_config_update_from_all_truncate_description(
        #[case] config: LibraryBenchmarkConfig,
    ) {
        let actual = LibraryBenchmarkConfig::default().update_from_all([Some(&config)]);
        assert_eq!(actual, config);
    }

    #[rstest]
    #[case::all_none(None, None, None)]
    #[case::some_and_none(Some(true), None, Some(true))]
    #[case::none_and_some(None, Some(true), Some(true))]
    #[case::some_and_some(Some(false), Some(true), Some(true))]
    #[case::some_and_some_value_does_not_matter(Some(true), Some(false), Some(false))]
    fn test_update_option(
        #[case] first: Option<bool>,
        #[case] other: Option<bool>,
        #[case] expected: Option<bool>,
    ) {
        assert_eq!(update_option(&first, &other), expected);
    }

    #[rstest]
    #[case::empty(vec![], &[], vec![])]
    #[case::empty_base(vec![], &["--a=yes"], vec!["--a=yes"])]
    #[case::no_flags(vec![], &["a=yes"], vec!["--a=yes"])]
    #[case::already_exists_single(vec!["--a=yes"], &["--a=yes"], vec!["--a=yes","--a=yes"])]
    #[case::already_exists_when_multiple(vec!["--a=yes", "--b=yes"], &["--a=yes"], vec!["--a=yes", "--b=yes", "--a=yes"])]
    fn test_raw_args_extend_ignore_flags(
        #[case] base: Vec<&str>,
        #[case] data: &[&str],
        #[case] expected: Vec<&str>,
    ) {
        let mut base = RawArgs(base.iter().map(std::string::ToString::to_string).collect());
        base.extend_ignore_flag(data.iter().map(std::string::ToString::to_string));

        assert_eq!(base.0.into_iter().collect::<Vec<String>>(), expected);
    }
}
