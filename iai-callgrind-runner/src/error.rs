use std::fmt::Display;
use std::io::stderr;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{ExitStatus, Output};

use version_compare::Cmp;

use crate::runner::common::ModulePath;
use crate::runner::tool::{ToolOutputPath, ValgrindTool};
use crate::util::write_all_to_stderr;

#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    InitError(String),
    VersionMismatch(version_compare::Cmp, String, String),
    LaunchError(PathBuf, String),
    ProcessError((String, Option<Output>, ExitStatus, Option<ToolOutputPath>)),
    InvalidCallgrindBoolArgument((String, String)),
    ParseError((PathBuf, String)),
    RegressionError(bool),
    EnvironmentVariableError((String, String)),
    SandboxError(String),
    BenchmarkError(ValgrindTool, ModulePath, String),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InitError(message) => {
                let runner_version = env!("CARGO_PKG_VERSION").to_owned();
                write!(
                    f,
                    "Failed to initialize iai-callgrind-runner: {message}\n\nDetected version of \
                     iai-callgrind-runner is {runner_version}. This error can be caused by a \
                     version mismatch between iai-callgrind and iai-callgrind-runner. If you \
                     updated the library (iai-callgrind) in your Cargo.toml file, the binary \
                     (iai-callgrind-runner) needs to be updated to the same version and vice \
                     versa."
                )
            }
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
                write!(f, "Error launching '{}': {message}", exec.display())
            }
            Self::ProcessError((process, output, status, output_path)) => {
                if let Some(output_path) = output_path {
                    output_path
                        .dump_log(log::Level::Error, &mut stderr())
                        .expect("Printing error output should succeed");
                }
                if let Some(output) = output {
                    write_all_to_stderr(&output.stderr);
                }

                if let Some(code) = status.code() {
                    write!(f, "Error running '{process}': Exit code was: '{code}'")
                } else if let Some(signal) = status.signal() {
                    write!(
                        f,
                        "Error running '{process}': Terminated by a signal '{signal}'"
                    )
                } else {
                    write!(f, "Error running '{process}': Terminated abnormally")
                }
            }
            Self::InvalidCallgrindBoolArgument((option, value)) => {
                write!(
                    f,
                    "Invalid callgrind argument for {option}: '{value}'. Valid values are 'yes' \
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
            Self::SandboxError(message) => {
                write!(f, "Error in sandbox: {message}")
            }
            Self::BenchmarkError(tool, module_path, message) => {
                write!(f, "Error in {tool} benchmark {module_path}: {message}")
            }
        }
    }
}
