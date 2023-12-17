use core::panic;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::{tempdir, TempDir};

pub const VALGRIND_WRAPPER: &str = env!("CARGO_BIN_EXE_valgrind-wrapper");
pub const FIXTURES_DIR: &str = env!("CLIENT_REQUEST_TESTS_FIXTURES");

fn find_runner() -> Option<String> {
    for (key, value) in std::env::vars() {
        if key.starts_with("CARGO_TARGET_") && key.ends_with("_RUNNER") && !value.is_empty() {
            return Some(value);
        }
    }
    None
}

pub fn get_test_bin_path(name: &str) -> PathBuf {
    PathBuf::from(VALGRIND_WRAPPER).parent().unwrap().join(name)
}

pub fn get_command<T>(path: T) -> Command
where
    T: AsRef<Path>,
{
    if let Some(runner) = find_runner() {
        let mut runner = runner.split_whitespace();
        let mut cmd = Command::new(runner.next().unwrap());
        for arg in runner {
            cmd.arg(arg);
        }
        cmd.arg(path.as_ref());
        cmd
    } else {
        Command::new(path.as_ref())
    }
}

pub fn get_test_bin_command<T>(name: T) -> Command
where
    T: AsRef<str>,
{
    let path = PathBuf::from(VALGRIND_WRAPPER)
        .parent()
        .unwrap()
        .join(name.as_ref());
    get_command(path)
}

pub fn get_valgrind_wrapper_command() -> Command {
    Command::new(VALGRIND_WRAPPER)
}

pub fn get_fixture_path(name: &str) -> PathBuf {
    PathBuf::from(FIXTURES_DIR).join(name)
}

pub fn get_fixture_as_string(name: &str) -> String {
    let mut file = File::open(get_fixture_path(name))
        .unwrap_or_else(|_| panic!("Opening fixture '{name}' should succeed"));

    let mut buf = String::new();
    file.read_to_string(&mut buf)
        .unwrap_or_else(|_| panic!("Reading content of fixture '{name}' should succeed"));

    buf
}

pub fn get_sandbox() -> TempDir {
    tempdir().expect("Creating sandbox directory failed")
}
