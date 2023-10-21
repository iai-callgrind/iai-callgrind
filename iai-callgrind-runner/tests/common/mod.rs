use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::Result;
use iai_callgrind_runner::runner::callgrind::CallgrindOutput;
use serde::{Deserialize, Serialize};

pub const FIXTURES_ROOT: &str = "tests/fixtures";

pub struct Fixtures;

impl Fixtures {
    pub fn get_path_of<T>(name: T) -> PathBuf
    where
        T: AsRef<Path>,
    {
        let path = Fixtures::get_path().join(name);
        if !path.exists() {
            panic!("Fixtures path '{}' does not exist", path.display());
        }
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

    pub fn get_callgrind_output<T>(path: T) -> CallgrindOutput
    where
        T: AsRef<Path>,
    {
        CallgrindOutput::from_existing(Fixtures::get_path_of(path)).unwrap()
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
