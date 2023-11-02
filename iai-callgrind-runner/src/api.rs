//! The api contains all elements which the `runner` can understand
use std::ffi::OsString;
use std::fmt::Display;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arg {
    pub id: Option<String>,
    pub args: Vec<OsString>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assistant {
    pub id: String,
    pub name: String,
    pub bench: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BinaryBenchmark {
    pub config: BinaryBenchmarkConfig,
    pub groups: Vec<BinaryBenchmarkGroup>,
    pub command_line_args: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BinaryBenchmarkConfig {
    pub sandbox: Option<bool>,
    pub fixtures: Option<Fixtures>,
    pub env_clear: Option<bool>,
    pub current_dir: Option<PathBuf>,
    pub entry_point: Option<String>,
    pub exit_with: Option<ExitWith>,
    pub raw_callgrind_args: RawArgs,
    pub envs: Vec<(OsString, Option<OsString>)>,
    pub flamegraph: Option<FlamegraphConfig>,
    pub regression: Option<RegressionConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BinaryBenchmarkGroup {
    pub id: Option<String>,
    pub cmd: Option<Cmd>,
    pub config: Option<BinaryBenchmarkConfig>,
    pub benches: Vec<Run>,
    pub assists: Vec<Assistant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cmd {
    pub display: String,
    pub cmd: String,
}

// TODO: CLEANUP
// #[derive(Debug, Clone, Default, Serialize, Deserialize)]
// pub struct DhatConfig {
//     pub enable: Option<bool>,
//     pub raw_args: RawArgs,
// }

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

/// All `EventKind`s callgrind produces and additionally some derived events
///
/// Depending on the options passed to Callgrind, these are the events that Callgrind can produce.
/// See the [Callgrind
/// documentation](https://valgrind.org/docs/manual/cl-manual.html#cl-manual.options) for details.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LibraryBenchmark {
    pub config: LibraryBenchmarkConfig,
    pub groups: Vec<LibraryBenchmarkGroup>,
    pub command_line_args: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LibraryBenchmarkBench {
    pub id: Option<String>,
    pub bench: String,
    pub args: Option<String>,
    pub config: Option<LibraryBenchmarkConfig>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LibraryBenchmarkBenches {
    pub config: Option<LibraryBenchmarkConfig>,
    pub benches: Vec<LibraryBenchmarkBench>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LibraryBenchmarkConfig {
    pub env_clear: Option<bool>,
    pub raw_callgrind_args: RawArgs,
    pub envs: Vec<(OsString, Option<OsString>)>,
    pub flamegraph: Option<FlamegraphConfig>,
    pub regression: Option<RegressionConfig>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LibraryBenchmarkGroup {
    pub id: Option<String>,
    pub config: Option<LibraryBenchmarkConfig>,
    pub benches: Vec<LibraryBenchmarkBenches>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawArgs(pub Vec<String>);

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RegressionConfig {
    pub limits: Vec<(EventKind, f64)>,
    pub fail_fast: Option<bool>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Run {
    pub cmd: Option<Cmd>,
    pub args: Vec<Arg>,
    pub config: BinaryBenchmarkConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub kind: ValgrindTool,
    pub enable: Option<bool>,
    pub raw_args: RawArgs,
    pub outfile_modifier: Option<String>,
    pub show_log: Option<bool>,
}

// TODO: MANAGE out files of
// bb-out-file, massif-out-file, dhat-out-file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValgrindTool {
    Memcheck,
    Helgrind,
    DRD,
    Massif,
    DHAT,
    BBV,
}

impl BinaryBenchmarkConfig {
    pub fn update_from_all<'a, T>(mut self, others: T) -> Self
    where
        T: IntoIterator<Item = Option<&'a Self>>,
    {
        for other in others.into_iter().flatten() {
            self.sandbox = update_option(&self.sandbox, &other.sandbox);
            self.fixtures = update_option(&self.fixtures, &other.fixtures);
            self.env_clear = update_option(&self.env_clear, &other.env_clear);
            self.current_dir = update_option(&self.current_dir, &other.current_dir);
            self.entry_point = update_option(&self.entry_point, &other.entry_point);
            self.exit_with = update_option(&self.exit_with, &other.exit_with);

            self.raw_callgrind_args
                .extend_ignore_flag(other.raw_callgrind_args.0.iter());

            self.envs.extend_from_slice(&other.envs);
            self.flamegraph = update_option(&self.flamegraph, &other.flamegraph);
            self.regression = update_option(&self.regression, &other.regression);
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
}

impl Default for Direction {
    fn default() -> Self {
        Self::BottomToTop
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
}

impl Display for EventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self:?}"))
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
            self.raw_callgrind_args
                .extend_ignore_flag(other.raw_callgrind_args.0.iter());
            self.env_clear = update_option(&self.env_clear, &other.env_clear);
            self.envs.extend_from_slice(&other.envs);
            self.flamegraph = update_option(&self.flamegraph, &other.flamegraph);
            self.regression = update_option(&self.regression, &other.regression);
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
}

impl RawArgs {
    pub fn new<I, T>(args: T) -> Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        args.into_iter().collect::<Self>()
    }

    pub fn extend_ignore_flag<I, T>(&mut self, args: T)
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        self.0.extend(args.into_iter().map(|s| {
            let string = s.as_ref();
            if string.starts_with("--") {
                string.to_owned()
            } else {
                format!("--{string}")
            }
        }));
    }

    pub fn from_command_line_args(args: Vec<String>) -> Self {
        let mut this = Self(Vec::default());
        if !args.is_empty() {
            let mut iter = args.into_iter();
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

pub fn update_option<T: Clone>(first: &Option<T>, other: &Option<T>) -> Option<T> {
    other.clone().or_else(|| first.clone())
}

#[cfg(test)]
mod tests {
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

    #[rstest]
    #[case::all_none(None, &[None], None)]
    #[case::default_is_overwritten_when_false(None, &[Some(false)], Some(false))]
    #[case::default_is_overwritten_when_true(None, &[Some(true)], Some(true))]
    #[case::some_is_overwritten_when_same_value(Some(true), &[Some(true)], Some(true))]
    #[case::some_is_overwritten_when_false(Some(false), &[Some(true)], Some(true))]
    #[case::some_is_not_overwritten_when_none(Some(true), &[None], Some(true))]
    #[case::multiple_when_none_then_ignored(Some(true), &[None, Some(false)], Some(false))]
    #[case::default_when_multiple_then_ignored(None, &[Some(true), None, Some(false)], Some(false))]
    fn test_library_benchmark_config_update_from_all_when_env_clear(
        #[case] base: Option<bool>,
        #[case] others: &[Option<bool>],
        #[case] expected: Option<bool>,
    ) {
        let base = LibraryBenchmarkConfig {
            env_clear: base,
            ..Default::default()
        };
        let others: Vec<LibraryBenchmarkConfig> = others
            .iter()
            .map(|o| LibraryBenchmarkConfig {
                env_clear: *o,
                ..Default::default()
            })
            .collect();

        let others = others
            .iter()
            .map(Some)
            .collect::<Vec<Option<&LibraryBenchmarkConfig>>>();

        assert_eq!(
            base.update_from_all(others),
            LibraryBenchmarkConfig {
                env_clear: expected,
                ..Default::default()
            }
        );
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
