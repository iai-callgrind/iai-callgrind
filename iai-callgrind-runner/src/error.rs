use std::path::PathBuf;
use std::process::Output;

pub type Result<T> = std::result::Result<T, IaiCallgrindError>;

#[derive(Debug, PartialEq, Eq)]
pub enum IaiCallgrindError {
    VersionMismatch(version_compare::Cmp, String, String),
    LaunchError(PathBuf, String),
    BenchmarkLaunchError(Output),
    Other(String),
    InvalidCallgrindBoolArgument((String, String)),
    ParseError((PathBuf, String)),
}
