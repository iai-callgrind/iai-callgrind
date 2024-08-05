use std::fmt::Display;
use std::path::PathBuf;

use super::meta::Metadata;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ModulePath(String);

impl ModulePath {
    pub fn new(path: &str) -> Self {
        Self(path.to_owned())
    }

    pub fn join(&self, path: &str) -> Self {
        let new = format!("{}::{path}", self.0);
        Self(new)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for ModulePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug)]
pub struct Config {
    pub package_dir: PathBuf,
    pub bench_file: PathBuf,
    // TODO: SHOULD BE A ModulePath
    pub module: String,
    pub bench_bin: PathBuf,
    pub meta: Metadata,
}
