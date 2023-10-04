use std::path::PathBuf;
use std::process::Output;

pub type Result<T> = std::result::Result<T, Error>;

// TODO: Improve this error. Use anyhow or thiserror. Implement From<std::error::Error>, ...
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    VersionMismatch(version_compare::Cmp, String, String),
    LaunchError(PathBuf, String),
    BenchmarkLaunchError(Output),
    Other(String),
    InvalidCallgrindBoolArgument((String, String)),
    ParseError((PathBuf, String)),
}
