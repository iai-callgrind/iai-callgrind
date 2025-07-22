//! The module responsible for the actual run of the benchmark

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output};

use anyhow::Result;
use log::{debug, error, log_enabled};

use super::config::ToolConfig;
use super::path::ToolOutputPath;
use crate::api::{self, ExitWith, Stream, ValgrindTool};
use crate::error::Error;
use crate::runner::args::NoCapture;
use crate::runner::bin_bench::Delay;
use crate::runner::common::{Assistant, ModulePath};
use crate::runner::meta::Metadata;
use crate::util::{self, resolve_binary_path};

/// The run options for the [`ToolCommand`]
#[derive(Debug, Default, Clone)]
pub struct RunOptions {
    /// Set the current directory of the [`ToolCommand`]
    pub current_dir: Option<PathBuf>,
    /// The optional [`Delay`] to apply to the command
    pub delay: Option<Delay>,
    /// If true, clear the environment variables
    pub env_clear: bool,
    /// The environment variables to pass into the [`ToolCommand`]
    pub envs: Vec<(OsString, OsString)>,
    /// Configuration of the expected exit code/signal
    pub exit_with: Option<ExitWith>,
    /// If present, execute the [`ToolCommand`] in a [`api::Sandbox`]
    pub sandbox: Option<api::Sandbox>,
    /// The `setup` assistant to run if present
    pub setup: Option<Assistant>,
    /// The `stderr`
    pub stderr: Option<api::Stdio>,
    /// The `stdin`
    pub stdin: Option<api::Stdin>,
    /// The `stdout`
    pub stdout: Option<api::Stdio>,
    /// The `teardown` assistant to run if present
    pub teardown: Option<Assistant>,
}

/// The final command to execute
pub struct ToolCommand {
    command: Command,
    nocapture: NoCapture,
    tool: ValgrindTool,
}

/// The tool specific [`Output`] of the [`ToolCommand`]
pub struct ToolOutput {
    /// The output if present
    pub output: Option<Output>,
    /// The valgrind tool
    pub tool: ValgrindTool,
}

impl ToolCommand {
    /// Create new `ToolCommand`
    pub fn new(tool: ValgrindTool, meta: &Metadata, nocapture: NoCapture) -> Self {
        Self {
            tool,
            nocapture,
            command: meta.into(),
        }
    }

    /// Clear the environment variables
    ///
    /// The `LD_PRELOAD` and `LD_LIBRARY_PATH` variables are skipped. If they are set there's
    /// usually a good reason for it.
    ///
    /// If the tool is `Memcheck`: In order to be able run `Memcheck` without errors, the `PATH`,
    /// `HOME` and `DEBUGINFOD_URLS` variables are skipped.
    pub fn env_clear(&mut self) -> &mut Self {
        debug!("{}: Clearing environment variables", self.tool.id());
        for (key, _) in std::env::vars() {
            match (key.as_str(), self.tool) {
                (key @ ("DEBUGINFOD_URLS" | "PATH" | "HOME"), ValgrindTool::Memcheck)
                | (key @ ("LD_PRELOAD" | "LD_LIBRARY_PATH"), _) => {
                    debug!(
                        "{}: Clearing environment variables: Skipping {key}",
                        self.tool.id()
                    );
                }
                _ => {
                    self.command.env_remove(key);
                }
            }
        }
        self
    }

    /// Run the `ToolCommand`
    #[allow(clippy::too_many_lines)]
    pub fn run(
        mut self,
        config: ToolConfig,
        executable: &Path,
        executable_args: &[OsString],
        run_options: RunOptions,
        output_path: &ToolOutputPath,
        module_path: &ModulePath,
        mut child: Option<Child>,
    ) -> Result<ToolOutput> {
        debug!(
            "{}: Running with executable '{}'",
            self.tool.id(),
            executable.display()
        );

        let RunOptions {
            env_clear,
            current_dir,
            exit_with,
            envs,
            stdin,
            stdout,
            stderr,
            ..
        } = run_options;

        if env_clear {
            debug!("Clearing environment variables");
            self.env_clear();
        }

        if let Some(dir) = current_dir {
            debug!(
                "{}: Setting current directory to '{}'",
                self.tool.id(),
                dir.display()
            );
            self.command.current_dir(dir);
        }

        let mut tool_args = config.args;
        tool_args.set_output_arg(output_path, Option::<&str>::None);
        tool_args.set_log_arg(output_path, Option::<&str>::None);

        let executable = resolve_binary_path(executable)?;
        let args = tool_args.to_vec();
        debug!(
            "{}: Arguments: {}",
            self.tool.id(),
            args.iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect::<Vec<String>>()
                .join(" ")
        );

        self.command
            .args(tool_args.to_vec())
            .arg(&executable)
            .args(executable_args)
            .envs(envs);

        if config.is_default {
            debug!("Applying --nocapture options");
            self.nocapture.apply(&mut self.command);
        }

        if let Some(stdin) = stdin {
            stdin
                .apply(&mut self.command, Stream::Stdin, child.as_mut())
                .map_err(|error| Error::BenchmarkError(self.tool, module_path.clone(), error))?;
        }

        if let Some(stdout) = stdout {
            stdout
                .apply(&mut self.command, Stream::Stdout)
                .map_err(|error| Error::BenchmarkError(self.tool, module_path.clone(), error))?;
        }

        if let Some(stderr) = stderr {
            stderr
                .apply(&mut self.command, Stream::Stderr)
                .map_err(|error| Error::BenchmarkError(self.tool, module_path.clone(), error))?;
        }

        let output = match self.nocapture {
            NoCapture::True | NoCapture::Stderr | NoCapture::Stdout if config.is_default => {
                self.command
                    .status()
                    .map_err(|error| {
                        Error::LaunchError(PathBuf::from("valgrind"), error.to_string()).into()
                    })
                    .and_then(|status| {
                        check_exit(
                            self.tool,
                            &executable,
                            None,
                            status,
                            &output_path.to_log_output(),
                            exit_with.as_ref(),
                        )
                    })?;
                None
            }
            _ => self
                .command
                .output()
                .map_err(|error| {
                    Error::LaunchError(PathBuf::from("valgrind"), error.to_string()).into()
                })
                .and_then(|output| {
                    let status = output.status;
                    check_exit(
                        self.tool,
                        &executable,
                        Some(output),
                        status,
                        &output_path.to_log_output(),
                        exit_with.as_ref(),
                    )
                })?,
        };

        if let Some(mut child) = child {
            debug!("Waiting for setup child process");
            let status = child.wait().expect("Setup child process should have run");
            if !status.success() {
                return Err(Error::ProcessError(
                    module_path.join("setup").to_string(),
                    None,
                    status,
                    None,
                )
                .into());
            }
        }

        output_path.sanitize()?;

        Ok(ToolOutput {
            tool: self.tool,
            output,
        })
    }
}

impl ToolOutput {
    /// Dump the tool output if the [`log::Level`] matches
    pub fn dump_log(&self, log_level: log::Level) {
        if let Some(output) = &self.output {
            if log_enabled!(log_level) {
                let (stdout, stderr) = (&output.stdout, &output.stderr);
                if !stdout.is_empty() {
                    log::log!(log_level, "{} output on stdout:", self.tool.id());
                    util::write_all_to_stderr(stdout);
                }
                if !stderr.is_empty() {
                    log::log!(log_level, "{} output on stderr:", self.tool.id());
                    util::write_all_to_stderr(stderr);
                }
            }
        }
    }
}

/// Check the exit code of the [`ToolCommand`] and verify it matches the expected [`ExitWith`]
pub fn check_exit(
    tool: ValgrindTool,
    executable: &Path,
    output: Option<Output>,
    status: ExitStatus,
    output_path: &ToolOutputPath,
    exit_with: Option<&ExitWith>,
) -> Result<Option<Output>> {
    let Some(status_code) = status.code() else {
        return Err(
            Error::ProcessError(tool.id(), output, status, Some(output_path.clone())).into(),
        );
    };

    match (status_code, exit_with) {
        (0i32, None | Some(ExitWith::Code(0i32) | ExitWith::Success)) => Ok(output),
        (0i32, Some(ExitWith::Code(code))) => {
            error!(
                "{}: Expected '{}' to exit with '{}' but it succeeded",
                tool.id(),
                executable.display(),
                code
            );
            Err(Error::ProcessError(tool.id(), output, status, Some(output_path.clone())).into())
        }
        (0i32, Some(ExitWith::Failure)) => {
            error!(
                "{}: Expected '{}' to fail but it succeeded",
                tool.id(),
                executable.display(),
            );
            Err(Error::ProcessError(tool.id(), output, status, Some(output_path.clone())).into())
        }
        (_, Some(ExitWith::Failure)) => Ok(output),
        (code, Some(ExitWith::Success)) => {
            error!(
                "{}: Expected '{}' to succeed but it terminated with '{}'",
                tool.id(),
                executable.display(),
                code
            );
            Err(Error::ProcessError(tool.id(), output, status, Some(output_path.clone())).into())
        }
        (actual_code, Some(ExitWith::Code(expected_code))) if actual_code == *expected_code => {
            Ok(output)
        }
        (actual_code, Some(ExitWith::Code(expected_code))) => {
            error!(
                "{}: Expected '{}' to exit with '{}' but it terminated with '{}'",
                tool.id(),
                executable.display(),
                expected_code,
                actual_code
            );
            Err(Error::ProcessError(tool.id(), output, status, Some(output_path.clone())).into())
        }
        _ => Err(Error::ProcessError(tool.id(), output, status, Some(output_path.clone())).into()),
    }
}
