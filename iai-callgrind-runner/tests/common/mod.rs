use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::Result;
use iai_callgrind_runner::runner::callgrind::CallgrindOutput;
use serde::{Deserialize, Serialize};

pub const FIXTURES_ROOT: &str = "tests/fixtures";

pub fn get_fixtures_path<T>(name: T) -> PathBuf
where
    T: AsRef<Path>,
{
    let root = get_project_root();
    if root.ends_with("iai-callgrind-runner") {
        root.join(FIXTURES_ROOT).join(name.as_ref())
    } else {
        root.join("iai-callgrind-runner")
            .join(FIXTURES_ROOT)
            .join(name.as_ref())
    }
}

pub fn get_callgrind_output<T>(path: T) -> CallgrindOutput
where
    T: AsRef<Path>,
{
    CallgrindOutput::from_existing(get_fixtures_path(path)).unwrap()
}

pub fn load_serialized<T, N>(name: N) -> Result<T, serde_yaml::Error>
where
    T: for<'de> Deserialize<'de>,
    N: AsRef<Path>,
{
    let file = File::open(get_fixtures_path(name)).unwrap();
    serde_yaml::from_reader::<File, T>(file)
}

#[allow(unused)]
pub fn save_serialized<T, N>(name: N, value: &T) -> Result<(), serde_yaml::Error>
where
    T: Serialize,
    N: AsRef<Path>,
{
    let file = File::create(get_fixtures_path(name)).unwrap();
    serde_yaml::to_writer(file, value)
}

pub fn get_project_root() -> PathBuf {
    let meta = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("Querying metadata of cargo workspace succeeds");

    meta.workspace_root.into_std_path_buf()
}

pub fn load_stacks<T>(path: T) -> Vec<String>
where
    T: AsRef<Path>,
{
    let path = get_fixtures_path(path);
    let reader = BufReader::new(File::open(path).unwrap());
    reader.lines().map(std::result::Result::unwrap).collect()
}

pub fn assert_parse_error<T>(file: &Path, result: Result<T>, message: &str)
where
    T: std::cmp::PartialEq + std::fmt::Debug,
{
    assert_eq!(
        result.unwrap_err().to_string(),
        format!("Error parsing file '{}': {message}", file.display())
    );
}
