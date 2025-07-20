use std::ffi::OsString;
use std::fmt::Display;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::Result;
use iai_callgrind_runner::api::ValgrindTool;
use iai_callgrind_runner::runner::summary::BaselineKind;
use iai_callgrind_runner::runner::tool::path::{ToolOutputPath, ToolOutputPathKind};
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};

pub const FIXTURES_ROOT: &str = "tests/fixtures";

pub struct Fixtures;

pub struct Runner {
    path: OsString,
    args: Vec<OsString>,
}

pub struct RunnerOutput(Output);

#[derive(Debug, Clone)]
pub struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Fixtures {
    pub fn get_path_of<T>(name: T) -> PathBuf
    where
        T: AsRef<Path>,
    {
        let path = Fixtures::get_path().join(name);
        assert!(
            path.exists(),
            "Fixtures path '{}' does not exist",
            path.display()
        );
        path
    }

    pub fn get_path() -> PathBuf {
        let root = get_project_root();
        if root.ends_with("iai-callgrind-runner") {
            root.join(FIXTURES_ROOT)
        } else {
            root.join("iai-callgrind-runner").join(FIXTURES_ROOT)
        }
    }

    pub fn get_tool_output_path(
        dir: &str,
        tool: ValgrindTool,
        kind: ToolOutputPathKind,
        name: &str,
    ) -> ToolOutputPath {
        ToolOutputPath {
            kind,
            tool,
            baseline_kind: BaselineKind::Old,
            dir: Fixtures::get_path().join(dir),
            name: name.to_owned(),
            modifiers: vec![],
        }
    }

    pub fn load_serialized<T, N>(name: N) -> Result<T, serde_yaml::Error>
    where
        T: for<'de> Deserialize<'de>,
        N: AsRef<Path>,
    {
        let file = File::open(Fixtures::get_path_of(name)).unwrap();
        serde_yaml::from_reader::<File, T>(file)
    }

    #[allow(unused)]
    pub fn save_serialized<T, N>(name: N, value: &T) -> Result<(), serde_yaml::Error>
    where
        T: Serialize,
        N: AsRef<Path>,
    {
        let file = File::create(Fixtures::get_path_of(name)).unwrap();
        serde_yaml::to_writer(file, value)
    }

    pub fn load_stacks<T>(path: T) -> Vec<String>
    where
        T: AsRef<Path>,
    {
        let path = Fixtures::get_path_of(path);
        let reader = BufReader::new(File::open(path).unwrap());
        reader.lines().map(std::result::Result::unwrap).collect()
    }
}

impl Runner {
    pub fn new() -> Self {
        let path = OsString::from(env!("CARGO_BIN_EXE_iai-callgrind-runner"));
        Self { path, args: vec![] }
    }

    pub fn run(&self) -> RunnerOutput {
        Command::new(&self.path)
            .args(&self.args)
            .env("IAI_CALLGRIND_COLOR", "never")
            .output()
            .map(RunnerOutput)
            .unwrap()
    }

    pub fn args(&mut self, args: &[&str]) -> &mut Self {
        for arg in args {
            self.args.push(OsString::from(arg));
        }

        self
    }
}

impl RunnerOutput {
    #[track_caller]
    #[allow(unused)]
    pub fn assert_stderr(&self, expected: &str) -> &Self {
        assert_eq!(std::str::from_utf8(&self.0.stderr).unwrap(), expected);
        self
    }

    #[track_caller]
    #[allow(unused)]
    pub fn assert_stdout(&self, expected: &str) -> &Self {
        assert_eq!(std::str::from_utf8(&self.0.stdout).unwrap(), expected);
        self
    }

    #[track_caller]
    pub fn assert_stderr_bytes(&self, expected: &[u8]) -> &Self {
        assert_eq!(&self.0.stderr, expected);
        self
    }

    #[track_caller]
    #[allow(unused)]
    pub fn assert_stdout_bytes(&self, expected: &[u8]) -> &Self {
        assert_eq!(&self.0.stdout, expected);
        self
    }

    #[track_caller]
    pub fn assert_stdout_is_empty(&self) -> &Self {
        assert!(
            self.0.stdout.is_empty(),
            "Expected stdout to be empty but was: {}",
            std::str::from_utf8(&self.0.stdout).unwrap()
        );
        self
    }
}

impl Version {
    pub fn new(version: &str) -> Self {
        let [major, minor, patch] = version
            .split('.')
            .map(|s| s.parse::<u64>().unwrap())
            .collect::<Vec<u64>>()[..]
        else {
            panic!("Invalid version: '{version}'");
        };

        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn increment(&mut self, part: &str) {
        match part {
            "major" => {
                self.major += 1;
            }
            "minor" => {
                self.minor += 1;
            }
            "patch" => {
                self.patch += 1;
            }
            _ => {
                panic!("Invalid part: {part}");
            }
        }
    }

    pub fn decrement(&mut self, part: &str) {
        match part {
            "major" => {
                self.major = self.major.saturating_sub(1);
            }
            "minor" => {
                self.minor = self.minor.saturating_sub(1);
            }
            "patch" => {
                self.patch = self.patch.saturating_sub(1);
            }
            _ => {
                panic!("Invalid part: {part}");
            }
        }
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub fn get_runner_version() -> Version {
    Version::new(env!("CARGO_PKG_VERSION"))
}

pub fn get_project_root() -> PathBuf {
    let meta = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("Querying metadata of cargo workspace succeeds");

    meta.workspace_root.into_std_path_buf()
}

#[track_caller]
pub fn assert_parse_error<T>(file: &Path, result: Result<T>, message: &str)
where
    T: std::cmp::PartialEq + std::fmt::Debug,
{
    assert_eq!(
        result.unwrap_err().to_string(),
        format!("Error parsing file '{}': {message}", file.display())
    );
}
