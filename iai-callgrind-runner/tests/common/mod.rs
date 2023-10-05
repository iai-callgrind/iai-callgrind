use std::fs::File;
use std::path::{Path, PathBuf};

use iai_callgrind_runner::runner::callgrind::CallgrindOutput;
use serde::{Deserialize, Serialize};

pub const FIXTURES_ROOT: &str = "tests/fixtures";

pub fn get_fixtures_path<T>(name: T) -> PathBuf
where
    T: AsRef<Path>,
{
    PathBuf::from(FIXTURES_ROOT).join(name.as_ref())
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
