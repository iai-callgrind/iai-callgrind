use std::fmt::Display;
use std::path::PathBuf;
use std::process::Output;

use version_compare::Cmp;

use crate::runner::write_all_to_stderr;

#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    VersionMismatch(version_compare::Cmp, String, String),
    LaunchError(PathBuf, String),
    BenchmarkLaunchError(Output),
    InvalidCallgrindBoolArgument((String, String)),
    ParseError((PathBuf, String)),
    RegressionError(bool),
    EnvironmentVariableError((String, String)),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VersionMismatch(cmp, runner_version, library_version) => match cmp {
                Cmp::Lt => write!(
                    f,
                    "iai-callgrind-runner ({runner_version}) is older than iai-callgrind \
                     ({library_version}). Please update iai-callgrind-runner by calling 'cargo \
                     install --version {library_version} iai-callgrind-runner'"
                ),
                Cmp::Gt => write!(
                    f,
                    "iai-callgrind-runner ({runner_version}) is newer than iai-callgrind \
                     ({library_version}). Please update iai-callgrind to '{runner_version}' in \
                     your Cargo.toml file"
                ),
                Cmp::Ne => write!(
                    f,
                    "No version information found for iai-callgrind but iai-callgrind-runner \
                     ({runner_version}) is >= '0.3.0'. Please update iai-callgrind to \
                     '{runner_version}' in your Cargo.toml file"
                ),
                _ => unreachable!(),
            },
            Self::LaunchError(exec, message) => {
                write!(f, "Error executing '{}': {message}", exec.display())
            }
            Self::BenchmarkLaunchError(output) => {
                write_all_to_stderr(&output.stderr);
                write!(
                    f,
                    "Error launching benchmark: Callgrind exit code was: {}",
                    output.status.code().unwrap()
                )
            }
            Self::InvalidCallgrindBoolArgument((option, value)) => {
                write!(
                    f,
                    "Invalid callgrind argument for --{option}: '{value}'. Valid values are 'yes' \
                     or 'no'"
                )
            }
            Self::ParseError((path, message)) => {
                write!(f, "Error parsing file '{}': {message}", path.display())
            }
            Self::RegressionError(is_fatal) => {
                if *is_fatal {
                    write!(f, "Performance has regressed. Aborting ...",)
                } else {
                    write!(f, "Performance has regressed.",)
                }
            }
            Self::EnvironmentVariableError((var, reason)) => {
                write!(f, "Failed parsing environment variable {var}: {reason}")
            }
        }
    }
}
