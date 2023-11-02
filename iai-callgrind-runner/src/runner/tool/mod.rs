pub mod args;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::Result;
use log::{debug, Level};

use self::args::ToolArgs;
use super::common::{ToolOutputPath, ValgrindTool};
use super::meta::Metadata;
use crate::error::Error;
use crate::runner::callgrind::RunOptions;
use crate::util::resolve_binary_path;
use crate::{api, util};

#[derive(Debug, Clone)]
pub struct ToolConfig {
    pub tool: ValgrindTool,
    pub is_enabled: bool,
    pub args: ToolArgs,
    pub outfile_modifier: Option<String>,
    // TODO: MAKE USE OF IT
    pub show_log: bool,
}

pub struct ToolCommand {
    tool: ValgrindTool,
    command: Command,
}

pub struct ToolOutput {
    pub tool: ValgrindTool,
    pub output: Output,
}

impl ToolCommand {
    pub fn new(tool: ValgrindTool, meta: &Metadata) -> Self {
        Self {
            tool,
            command: meta.into(),
        }
    }

    pub fn env_clear(&mut self) -> &mut Self {
        debug!("{}: Clearing environment variables", self.tool.id());
        for (key, _) in std::env::vars() {
            match (key.as_str(), self.tool) {
                (key @ ("DEBUGINFOD_URLS" | "PATH" | "HOME"), ValgrindTool::Memcheck) => {
                    debug!(
                        "{}: Clearing environment variables: Skipping {key}",
                        self.tool.id()
                    );
                }
                (key @ ("LD_PRELOAD" | "LD_LIBRARY_PATH"), _) => {
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

    pub fn run(
        mut self,
        config: ToolConfig,
        executable: &Path,
        executable_args: &[OsString],
        options: RunOptions,
        output_path: &ToolOutputPath,
    ) -> Result<ToolOutput> {
        debug!(
            "{}: Running with executable '{}'",
            self.tool.id(),
            executable.display()
        );

        let RunOptions {
            env_clear,
            current_dir,
            envs,
            ..
        } = options;

        if env_clear {
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
        tool_args.set_output_arg(output_path, config.outfile_modifier.as_ref());

        let executable = resolve_binary_path(executable)?;

        // TODO: CHECK EXIT like in callgrind ??
        let output = self
            .command
            .args(tool_args.to_vec())
            .arg(&executable)
            .args(executable_args)
            .envs(envs)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| -> anyhow::Error {
                Error::LaunchError(PathBuf::from("valgrind"), error.to_string()).into()
            })?;

        Ok(ToolOutput {
            tool: self.tool,
            output,
        })
    }
}

impl From<api::Tool> for ToolConfig {
    fn from(value: api::Tool) -> Self {
        let tool = value.kind.into();
        Self {
            tool,
            is_enabled: value.enable.unwrap_or(true),
            args: ToolArgs::from_raw_args(tool, value.raw_args),
            outfile_modifier: value.outfile_modifier,
            show_log: value.show_log.unwrap_or(false),
        }
    }
}

impl ToolOutput {
    pub fn dump_if(&self, log_level: Level) {
        if log::log_enabled!(log_level) {
            let (stdout, stderr) = (&self.output.stdout, &self.output.stderr);
            if !stdout.is_empty() {
                log::log!(log_level, "{} output on stdout:", self.tool.id());
                util::write_all_to_stdout(stdout);
            }
            if !stderr.is_empty() {
                log::log!(log_level, "{} output on stderr:", self.tool.id());
                util::write_all_to_stderr(stderr);
            }
        }
    }
}
