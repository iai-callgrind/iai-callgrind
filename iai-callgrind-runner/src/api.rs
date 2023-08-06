use std::ffi::OsString;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub raw_callgrind_args: Vec<String>,
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
