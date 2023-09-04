use std::path::PathBuf;
use std::process::Output;

pub type Result<T> = std::result::Result<T, IaiCallgrindError>;

pub enum IaiCallgrindError {
    VersionMismatch(version_compare::Cmp, String, String),
    LaunchError(PathBuf, std::io::Error),
    BenchmarkLaunchError(Output),
    Other(String),
}
