use std::ffi::OsString;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LibraryBenchmarkConfig {
    pub env_clear: Option<bool>,
    pub raw_callgrind_args: RawCallgrindArgs,
}

impl LibraryBenchmarkConfig {
    pub fn update(&mut self, other: &Self) -> &mut Self {
        self.raw_callgrind_args
            .raw_callgrind_args_iter(other.raw_callgrind_args.0.iter());
        self.env_clear = match (self.env_clear, other.env_clear) {
            (None, None) => None,
            (None, Some(v)) | (Some(v), None) => Some(v),
            (Some(_), Some(w)) => Some(w),
        };
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
    pub benches: Vec<Vec<Function>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Function {
    pub id: Option<String>,
    pub bench: String,
    pub args: Option<String>,
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
