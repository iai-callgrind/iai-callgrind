pub mod args;

use std::ffi::{OsStr, OsString};
use std::fmt::Display;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::{anyhow, Context, Result};
use log::{debug, error, Level};

use self::args::ToolArgs;
use super::meta::Metadata;
use crate::api::ExitWith;
use crate::error::Error;
use crate::util::{resolve_binary_path, truncate_str_utf8};
use crate::{api, util};

#[derive(Debug, Default, Clone)]
pub struct RunOptions {
    pub env_clear: bool,
    pub current_dir: Option<PathBuf>,
    pub entry_point: Option<String>,
    pub exit_with: Option<ExitWith>,
    pub envs: Vec<(OsString, OsString)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolConfig {
    pub tool: ValgrindTool,
    pub is_enabled: bool,
    pub args: ToolArgs,
    pub outfile_modifier: Option<String>,
    // TODO: MAKE USE OF IT
    pub show_log: bool,
}

pub struct ToolOutputPath {
    pub tool: ValgrindTool,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValgrindTool {
    Callgrind,
    Memcheck,
    Helgrind,
    DRD,
    Massif,
    DHAT,
    BBV,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolConfigs(pub Vec<ToolConfig>);

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
            exit_with,
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
            })
            .and_then(|output| check_exit(&executable, output, exit_with.as_ref()))?;

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

impl ToolConfigs {
    pub fn run(
        &self,
        meta: &Metadata,
        executable: &Path,
        executable_args: &[OsString],
        options: &RunOptions,
        output_path: &ToolOutputPath,
    ) -> Result<()> {
        for tool_config in self.0.iter().filter(|t| t.is_enabled) {
            let command = ToolCommand::new(tool_config.tool, meta);
            let output_path = output_path.to_tool_output(tool_config.tool);
            output_path.init();
            let output = command.run(
                tool_config.clone(),
                executable,
                executable_args,
                options.clone(),
                &output_path,
            )?;
            output.dump_if(log::Level::Info);
        }
        Ok(())
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

impl ToolOutputPath {
    pub fn new(tool: ValgrindTool, base_dir: &Path, module: &str, name: &str) -> Self {
        let current = base_dir;
        let module_path: PathBuf = module.split("::").collect();
        let sanitized_name = sanitize_filename::sanitize_with_options(
            name,
            sanitize_filename::Options {
                windows: false,
                truncate: false,
                replacement: "_",
            },
        );
        let file_name = PathBuf::from(format!(
            "{}.{}.out",
            // callgrind. + .out.old = 18 + 37 bytes headroom for extensions with more than 3
            // bytes. max length is usually 255 bytes
            tool.id(),
            truncate_str_utf8(&sanitized_name, 200)
        ));

        let path = current.join(base_dir).join(module_path).join(file_name);
        Self { tool, path }
    }

    pub fn from_existing<T>(tool: ValgrindTool, path: T) -> Result<Self>
    where
        T: Into<PathBuf>,
    {
        let path: PathBuf = path.into();
        if !path.is_file() {
            return Err(anyhow!(
                "The callgrind output file '{}' did not exist or is not a valid file",
                path.display()
            ));
        }
        Ok(Self { tool, path })
    }

    /// Initialize and create the output directory and organize files
    ///
    /// This method moves the old output to `$TOOL_ID.*.out.old`
    /// TODO: RETURN Result
    pub fn with_init(tool: ValgrindTool, base_dir: &Path, module: &str, name: &str) -> Self {
        let output = Self::new(tool, base_dir, module, name);
        output.init();
        output
    }

    // TODO: RETURN Result
    // TODO: MOVE all output files in case of a modifier
    pub fn init(&self) {
        std::fs::create_dir_all(self.path.parent().unwrap()).expect("Failed to create directory");

        if self.exists() {
            let old_output = self.to_old_output();
            // Already run this benchmark once; move last results to .old
            std::fs::copy(&self.path, old_output.path).unwrap();
        }
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn with_extension<T>(&self, extension: T) -> Self
    where
        T: AsRef<OsStr>,
    {
        Self {
            tool: self.tool,
            path: self.path.with_extension(extension),
        }
    }

    pub fn to_old_output(&self) -> Self {
        Self {
            tool: self.tool,
            path: self.path.with_extension("out.old"),
        }
    }

    pub fn to_tool_output(&self, tool: ValgrindTool) -> Self {
        let file_name: &str = std::str::from_utf8(
            self.path
                .file_name()
                .unwrap()
                .as_bytes()
                .strip_prefix(self.tool.id().as_bytes())
                .unwrap(),
        )
        .unwrap();
        let path = self
            .path
            .with_file_name(format!("{}{file_name}", tool.id()));
        Self { tool, path }
    }

    pub fn open(&self) -> Result<File> {
        File::open(&self.path).with_context(|| {
            format!(
                "Error opening callgrind output file '{}'",
                self.path.display()
            )
        })
    }

    pub fn lines(&self) -> Result<impl Iterator<Item = String>> {
        let file = self.open()?;
        Ok(BufReader::new(file)
            .lines()
            .map(std::result::Result::unwrap))
    }

    pub fn as_path(&self) -> &Path {
        &self.path
    }
}

impl Display for ToolOutputPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.path.display()))
    }
}

impl ValgrindTool {
    /// Return the id used by the `valgrind --tool` option
    pub fn id(&self) -> String {
        match self {
            ValgrindTool::DHAT => "dhat".to_owned(),
            ValgrindTool::Callgrind => "callgrind".to_owned(),
            ValgrindTool::Memcheck => "memcheck".to_owned(),
            ValgrindTool::Helgrind => "helgrind".to_owned(),
            ValgrindTool::DRD => "drd".to_owned(),
            ValgrindTool::Massif => "massif".to_owned(),
            ValgrindTool::BBV => "exp-bbv".to_owned(),
        }
    }

    pub fn has_output_file(&self) -> bool {
        matches!(
            self,
            ValgrindTool::Callgrind | ValgrindTool::DHAT | ValgrindTool::BBV | ValgrindTool::Massif
        )
    }
}

impl From<api::ValgrindTool> for ValgrindTool {
    fn from(value: api::ValgrindTool) -> Self {
        match value {
            api::ValgrindTool::Memcheck => ValgrindTool::Memcheck,
            api::ValgrindTool::Helgrind => ValgrindTool::Helgrind,
            api::ValgrindTool::DRD => ValgrindTool::DRD,
            api::ValgrindTool::Massif => ValgrindTool::Massif,
            api::ValgrindTool::DHAT => ValgrindTool::DHAT,
            api::ValgrindTool::BBV => ValgrindTool::BBV,
        }
    }
}

pub fn check_exit(
    executable: &Path,
    output: Output,
    exit_with: Option<&ExitWith>,
) -> Result<Output> {
    let status_code = if let Some(code) = output.status.code() {
        code
    } else {
        return Err(Error::BenchmarkLaunchError(output).into());
    };

    match (status_code, exit_with) {
        (0i32, None | Some(ExitWith::Code(0i32) | ExitWith::Success)) => Ok(output),
        (0i32, Some(ExitWith::Code(code))) => {
            error!(
                "Expected benchmark '{}' to exit with '{}' but it succeeded",
                executable.display(),
                code
            );
            Err(Error::BenchmarkLaunchError(output).into())
        }
        (0i32, Some(ExitWith::Failure)) => {
            error!(
                "Expected benchmark '{}' to fail but it succeeded",
                executable.display(),
            );
            Err(Error::BenchmarkLaunchError(output).into())
        }
        (_, Some(ExitWith::Failure)) => Ok(output),
        (code, Some(ExitWith::Success)) => {
            error!(
                "Expected benchmark '{}' to succeed but it exited with '{}'",
                executable.display(),
                code
            );
            Err(Error::BenchmarkLaunchError(output).into())
        }
        (actual_code, Some(ExitWith::Code(expected_code))) if actual_code == *expected_code => {
            Ok(output)
        }
        (actual_code, Some(ExitWith::Code(expected_code))) => {
            error!(
                "Expected benchmark '{}' to exit with '{}' but it exited with '{}'",
                executable.display(),
                expected_code,
                actual_code
            );
            Err(Error::BenchmarkLaunchError(output).into())
        }
        _ => Err(Error::BenchmarkLaunchError(output).into()),
    }
}
