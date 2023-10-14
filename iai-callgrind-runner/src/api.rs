use std::path::PathBuf;
/// The api contains all elements which the `runner` can understand
use std::{ffi::OsString, fmt::Display};

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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BinaryBenchmarkConfig {
    pub sandbox: Option<bool>,
    pub fixtures: Option<Fixtures>,
    pub env_clear: Option<bool>,
    pub current_dir: Option<PathBuf>,
    pub entry_point: Option<String>,
    pub exit_with: Option<ExitWith>,
    pub raw_callgrind_args: RawCallgrindArgs,
    pub envs: Vec<(OsString, Option<OsString>)>,
    pub flamegraph: Option<FlamegraphConfig>,
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
///
/// TODO: DOCUMENT
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    TopToBottom,
    BottomToTop,
}

/// TODO: DOCUMENT
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    // always there
    Ir,
    // --collect-systime
    SysCount,
    SysTime,
    SysCpuTime,
    // --collect-bus
    Ge,
    // --cache-sim
    Dr,
    Dw,
    I1mr,
    ILmr,
    D1mr,
    DLmr,
    D1mw,
    DLmw,
    // --branch-sim
    Bc,
    Bcm,
    Bi,
    Bim,
    // --simulate-wb
    ILdmr,
    DLdmr,
    DLdmw,
    // --cachuse
    AcCost1,
    AcCost2,
    SpLoss1,
    SpLoss2,
    // Defined by us
    L1hits,
    LLhits,
    RamHits,
    TotalRW,
    EstimatedCycles,
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

/// TODO: DOCUMENT
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlamegraphKind {
    Regular,
    Differential,
    All,
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
    pub raw_callgrind_args: RawCallgrindArgs,
    pub envs: Vec<(OsString, Option<OsString>)>,
    pub flamegraph: Option<FlamegraphConfig>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LibraryBenchmarkGroup {
    pub id: Option<String>,
    pub config: Option<LibraryBenchmarkConfig>,
    pub benches: Vec<LibraryBenchmarkBenches>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawCallgrindArgs(pub Vec<String>);

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Run {
    pub cmd: Option<Cmd>,
    pub args: Vec<Arg>,
    pub config: BinaryBenchmarkConfig,
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
                .extend(other.raw_callgrind_args.0.iter());

            self.envs.extend_from_slice(&other.envs);
            self.flamegraph = update_option(&self.flamegraph, &other.flamegraph);
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
                .extend(other.raw_callgrind_args.0.iter());
            self.env_clear = update_option(&self.env_clear, &other.env_clear);
            self.envs.extend_from_slice(&other.envs);
            self.flamegraph = update_option(&self.flamegraph, &other.flamegraph);
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

impl RawCallgrindArgs {
    pub fn new<I, T>(args: T) -> Self
    where
        I: AsRef<str>,
        T: IntoIterator<Item = I>,
    {
        args.into_iter().collect::<Self>()
    }

    pub fn extend<I, T>(&mut self, args: T) -> &mut Self
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
        self
    }

    pub fn extend_from_command_line_args(&mut self, other: &[String]) {
        // The last argument is usually --bench. This argument comes from cargo and does not belong
        // to the arguments passed from the main macro. So, we're removing it if it is there.
        if other.ends_with(&["--bench".to_owned()]) {
            self.0.extend_from_slice(&other[..other.len() - 1]);
        } else {
            self.0.extend_from_slice(other);
        }
    }
}

impl<I> FromIterator<I> for RawCallgrindArgs
where
    I: AsRef<str>,
{
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        let mut this = Self::default();
        this.extend(iter);
        this
    }
}

fn update_option<T: Clone>(first: &Option<T>, other: &Option<T>) -> Option<T> {
    match (first, other) {
        (None, None) => None,
        (None, Some(v)) | (Some(v), None) => Some(v.clone()),
        (Some(_), Some(w)) => Some(w.clone()),
    }
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
}
