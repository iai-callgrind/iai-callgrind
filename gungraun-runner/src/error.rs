//! The module containing the crate main [`Error`] type

use std::fmt::Display;
use std::io::stderr;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{ExitStatus, Output};

use version_compare::Cmp;

use crate::api::ValgrindTool;
use crate::runner::common::ModulePath;
use crate::runner::format::Header;
use crate::runner::tool::path::ToolOutputPath;
use crate::util::write_all_to_stderr;

/// The main Gungraun error type
#[derive(Debug, PartialEq, Clone, Eq)]
pub enum Error {
    /// A error during setup of a benchmark.
    ///
    /// `BenchmarkError(ValgrindTool, ModulePath, message)`
    BenchmarkError(ValgrindTool, ModulePath, String),
    /// An error within the UI configuration structs but transpiring in the runner
    ///
    /// `ConfigurationError(ModulePath, benchmark_id, message)`
    ConfigurationError(ModulePath, Option<String>, String),
    /// An error during the initialization of the runner
    ///
    /// `InitError(message)`
    InitError(String),
    /// An invalid command-line argument value when only `yes` or `no` is allowed
    ///
    /// `InvalidBoolArgument(option_name, value)`
    InvalidBoolArgument(String, String),
    /// The error when trying to start an external [`std::process::Command`] fails
    ///
    /// `LaunchError(executable_path, message)`
    LaunchError(PathBuf, String),
    /// The generic error when parsing of a tools log- or output file fails
    ///
    /// `ParseError(file_path, message)`
    ParseError(PathBuf, String),
    /// The error after a successful launch of an external [`std::process::Command`]
    ///
    /// ```text
    /// ProcessError(
    ///     process_name,
    ///     std::process::Output,
    ///     std::process::ExitStatus,
    ///     ToolOutputPath
    /// )
    /// ```
    ProcessError(String, Option<Output>, ExitStatus, Option<ToolOutputPath>),
    /// If a regression check fails a `RegressionError` is issued
    ///
    /// `RegressionError(is_fatal)`, `is_fatal` needs to be true if the error should lead to an
    /// immediate exit of the runner
    RegressionError(bool),
    /// The error when setting up the [`crate::runner::common::Sandbox`] fails
    ///
    /// `SandboxError(message)`
    SandboxError(String),
    /// A version mismatch between the runner and the UI
    ///
    /// `VersionMismatch(Cmp, runner_version, library_version)`
    VersionMismatch(version_compare::Cmp, String, String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: MIGRATION: Double check these error messages
        match self {
            Self::InitError(message) => {
                let runner_version = env!("CARGO_PKG_VERSION").to_owned();
                write!(
                    f,
                    "Failed to initialize gungraun-runner: {message}\n\nDetected version of \
                     gungraun-runner is {runner_version}. This error can be caused by a version \
                     mismatch between iai-callgrind and gungraun-runner. If you updated the \
                     library (iai-callgrind) in your Cargo.toml file, the binary \
                     (gungraun-runner) needs to be updated to the same version and vice versa."
                )
            }
            Self::VersionMismatch(cmp, runner_version, library_version) => match cmp {
                Cmp::Lt => write!(
                    f,
                    "gungraun-runner ({runner_version}) is older than iai-callgrind \
                     ({library_version}). Please update gungraun-runner by calling 'cargo install \
                     --version {library_version} gungraun-runner'"
                ),
                Cmp::Gt => write!(
                    f,
                    "gungraun-runner ({runner_version}) is newer than iai-callgrind \
                     ({library_version}). Please update iai-callgrind to '{runner_version}' in \
                     your Cargo.toml file"
                ),
                Cmp::Ne => write!(
                    f,
                    "No version information found for iai-callgrind but gungraun-runner \
                     ({runner_version}) is >= '0.3.0'. Please update iai-callgrind to \
                     '{runner_version}' in your Cargo.toml file"
                ),
                _ => unreachable!(),
            },
            Self::LaunchError(exec, message) => {
                write!(f, "Error launching '{}': {message}", exec.display())
            }
            Self::ProcessError(process, output, status, output_path) => {
                if let Some(output_path) = output_path {
                    let _ = output_path.dump_log(log::Level::Error, &mut stderr());
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
            Self::InvalidBoolArgument(option, value) => {
                write!(
                    f,
                    "Invalid argument for {option}: '{value}'. Valid values are 'yes' or 'no'"
                )
            }
            Self::ParseError(path, message) => {
                write!(f, "Error parsing file '{}': {message}", path.display())
            }
            Self::RegressionError(is_fatal) => {
                if *is_fatal {
                    write!(
                        f,
                        "Performance has regressed. Fail-fast is set. Aborting ...",
                    )
                } else {
                    write!(f, "Performance has regressed.",)
                }
            }
            Self::SandboxError(message) => {
                write!(f, "Error in sandbox: {message}")
            }
            Self::BenchmarkError(tool, module_path, message) => {
                write!(f, "Error in {tool} benchmark {module_path}: {message}")
            }
            Self::ConfigurationError(module_path, id, message) => {
                let header = Header::without_description(module_path, id.clone());
                write!(f, "Misconfiguration in: {header}\nCaused by:\n  {message}",)
            }
        }
    }
}

impl std::error::Error for Error {}
