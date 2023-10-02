use std::path::{Path, PathBuf};

use iai_callgrind_runner::runner::callgrind::CallgrindOutput;

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
    CallgrindOutput::new(get_fixtures_path(path))
}
