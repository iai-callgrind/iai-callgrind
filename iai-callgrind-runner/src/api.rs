use std::ffi::OsString;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawCallgrindArgs(pub Vec<String>);

impl RawCallgrindArgs {
    pub fn new<I, T>(args: T) -> Self
    where
        I: AsRef<str>,
        T: AsRef<[I]>,
    {
        args.as_ref().iter().collect::<Self>()
    }

    pub fn raw_callgrind_args_iter<I, T>(&mut self, args: T) -> &mut Self
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
}

impl<I> FromIterator<I> for RawCallgrindArgs
where
    I: AsRef<str>,
{
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        let mut this = Self::default();
        this.raw_callgrind_args_iter(iter);
        this
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub raw_callgrind_args: RawCallgrindArgs,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryBenchmarkConfig {
    pub env_clear: Option<bool>,
    pub raw_callgrind_args: RawCallgrindArgs,
    pub envs: Vec<(OsString, Option<OsString>)>,
}

impl LibraryBenchmarkConfig {
    pub fn update_from_all<'a, T>(mut self, others: T) -> Self
    where
        T: IntoIterator<Item = Option<&'a Self>>,
    {
        for other in others.into_iter().flatten() {
            self.raw_callgrind_args
                .raw_callgrind_args_iter(other.raw_callgrind_args.0.iter());
            self.env_clear = match (self.env_clear, other.env_clear) {
                (None, None) => None,
                (None, Some(v)) | (Some(v), None) => Some(v),
                (Some(_), Some(w)) => Some(w),
            };
            self.envs.extend_from_slice(&other.envs);
        }
        self
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LibraryBenchmark {
    pub config: LibraryBenchmarkConfig,
    pub groups: Vec<LibraryBenchmarkGroup>,
    pub command_line_args: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LibraryBenchmarkGroup {
    pub id: Option<String>,
    pub config: Option<LibraryBenchmarkConfig>,
    pub benches: Vec<LibraryBenchmarkBenches>,
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

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BinaryBenchmark {
    pub config: Config,
    pub groups: Vec<BinaryBenchmarkGroup>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BinaryBenchmarkGroup {
    pub id: Option<String>,
    pub cmd: Option<Cmd>,
    pub fixtures: Option<Fixtures>,
    pub sandbox: bool,
    pub benches: Vec<Run>,
    pub assists: Vec<Assistant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assistant {
    pub id: String,
    pub name: String,
    pub bench: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arg {
    pub id: Option<String>,
    pub args: Vec<OsString>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixtures {
    pub path: PathBuf,
    pub follow_symlinks: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cmd {
    pub display: String,
    pub cmd: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Run {
    pub cmd: Option<Cmd>,
    pub args: Vec<Arg>,
    pub opts: Option<Options>,
    pub envs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Options {
    pub env_clear: bool,
    pub current_dir: Option<PathBuf>,
    pub entry_point: Option<String>,
    pub exit_with: Option<ExitWith>,
}

impl Options {
    pub fn new(
        env_clear: bool,
        current_dir: Option<PathBuf>,
        entry_point: Option<String>,
        exit_with: Option<ExitWith>,
    ) -> Self {
        Self {
            env_clear,
            current_dir,
            entry_point,
            exit_with,
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            env_clear: true,
            current_dir: Option::default(),
            entry_point: Option::default(),
            exit_with: Option::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExitWith {
    Success,
    Failure,
    Code(i32),
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
