use std::path::PathBuf;
use std::process::Output;

/// TODO: DOCUMENT
pub enum IaiCallgrindError {
    /// TODO: DOCUMENT
    VersionMismatch(version_compare::Cmp, String, String),
    /// TODO: DOCUMENT
    LaunchError(PathBuf, std::io::Error),
    /// TODO: DOCUMENT
    BenchmarkLaunchError(Output),
    /// TODO: DOCUMENT
    Other(String),
}
