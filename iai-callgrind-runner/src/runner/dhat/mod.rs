use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::Result;
use log::{debug, info, Level};

use super::meta::Metadata;
use crate::error::Error;
use crate::runner::{write_all_to_stderr, write_all_to_stdout};
use crate::util::resolve_binary_path;

pub struct DhatCommand {
    command: Command,
}

#[derive(Debug, Default, Clone)]
pub struct DhatOptions {
    pub env_clear: bool,
    pub current_dir: Option<PathBuf>,
    // pub exit_with: Option<ExitWith>,
    pub envs: Vec<(OsString, OsString)>,
}

#[derive(Debug, Clone)]
pub struct DhatOutput(PathBuf);

impl DhatCommand {
    pub fn new(meta: &Metadata) -> Self {
        let command = meta.valgrind_wrapper.as_ref().map_or_else(
            || {
                let meta_cmd = &meta.valgrind;
                let mut cmd = Command::new(&meta_cmd.bin);
                cmd.args(&meta_cmd.args);
                cmd
            },
            |meta_cmd| {
                let mut cmd = Command::new(&meta_cmd.bin);
                cmd.args(&meta_cmd.args);
                cmd
            },
        );
        Self { command }
    }

    pub fn run(
        self,
        executable: &Path,
        executable_args: &[OsString],
        options: DhatOptions,
        output: &DhatOutput,
    ) -> Result<()> {
        let mut command = self.command;
        // TODO: DhatArgs struct
        let mut dhat_args = vec![];

        debug!(
            "Running callgrind with executable '{}'",
            executable.display()
        );
        let DhatOptions {
            env_clear,
            current_dir,
            envs,
        } = options;

        if env_clear {
            debug!("Clearing environment variables");
            command.env_clear();
        }
        if let Some(dir) = current_dir {
            debug!("Setting current directory to '{}'", dir.display());
            command.current_dir(dir);
        }
        dhat_args.push(format!("--dhat-out-file={}", output.0.display()));

        let executable = resolve_binary_path(executable)?;

        // TODO: CHECK EXIT like in callgrind
        let (stdout, stderr) = command
            .arg("--tool=dhat")
            .args(dhat_args)
            .arg(&executable)
            .args(executable_args)
            .envs(envs)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map(|output| (output.stdout, output.stderr))
            .map_err(|error| -> anyhow::Error {
                Error::LaunchError(PathBuf::from("valgrind"), error.to_string()).into()
            })?;

        if !stdout.is_empty() {
            info!("Dhat output on stdout:");
            if log::log_enabled!(Level::Info) {
                write_all_to_stdout(&stdout);
            }
        }
        if !stderr.is_empty() {
            info!("Dhat output on stderr:");
            if log::log_enabled!(Level::Info) {
                write_all_to_stderr(&stderr);
            }
        }
        Ok(())
    }
}
