use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::Result;
use log::debug;

use super::common::ToolOutputPath;
use super::meta::Metadata;
use crate::error::Error;
use crate::runner::callgrind::RunOptions;
use crate::util::resolve_binary_path;

pub struct ToolCommand {
    command: Command,
}

pub struct ToolOutput(Output);

impl ToolCommand {
    pub fn new(meta: &Metadata) -> Self {
        Self {
            command: meta.into(),
        }
    }

    pub fn run(
        self,
        executable: &Path,
        executable_args: &[OsString],
        options: RunOptions,
        output_path: &ToolOutputPath,
    ) -> Result<ToolOutput> {
        let mut command = self.command;
        // TODO: DhatArgs struct
        let mut dhat_args = vec![];

        debug!("Running dhat with executable '{}'", executable.display());
        let RunOptions {
            env_clear,
            current_dir,
            envs,
            ..
        } = options;

        if env_clear {
            debug!("Clearing environment variables");
            command.env_clear();
        }
        if let Some(dir) = current_dir {
            debug!("Setting current directory to '{}'", dir.display());
            command.current_dir(dir);
        }
        dhat_args.push(format!("--dhat-out-file={}", output_path.path.display()));

        let executable = resolve_binary_path(executable)?;

        // TODO: CHECK EXIT like in callgrind ??
        let output = command
            .arg("--tool=dhat")
            .args(dhat_args)
            .arg(&executable)
            .args(executable_args)
            .envs(envs)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|error| -> anyhow::Error {
                Error::LaunchError(PathBuf::from("valgrind"), error.to_string()).into()
            })?;

        // TODO: CLEANUP
        // if !stdout.is_empty() {
        //     info!("Dhat output on stdout:");
        //     if log::log_enabled!(Level::Info) {
        //         write_all_to_stdout(&stdout);
        //     }
        // }
        // if !stderr.is_empty() {
        //     info!("Dhat output on stderr:");
        //     if log::log_enabled!(Level::Info) {
        //         write_all_to_stderr(&stderr);
        //     }
        // }
        Ok(ToolOutput(output))
    }
}
