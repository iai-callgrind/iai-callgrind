// spell-checker: ignore extbase extbasename extold
pub mod args;
pub mod format;
pub mod logfile_parser;

use std::ffi::OsString;
use std::fmt::Display;
use std::fs::File;
use std::io::{stderr, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use log::{debug, error, log_enabled, Level};
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use self::args::ToolArgs;
use self::format::LogfileSummaryFormatter;
use self::logfile_parser::{LogfileParser, LogfileSummary};
use super::dhat::logfile_parser::LogfileParser as DhatLogfileParser;
use super::format::{tool_headline, OutputFormat};
use super::meta::Metadata;
use super::summary::{BaselineKind, ToolSummary};
use crate::api::{self, ExitWith};
use crate::error::Error;
use crate::util::{self, make_relative, resolve_binary_path, truncate_str_utf8};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolOutputPath {
    pub kind: ToolOutputPathKind,
    pub tool: ValgrindTool,
    pub baseline_kind: BaselineKind,
    pub dir: PathBuf,
    pub name: String,
    pub modifiers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolOutputPathKind {
    Out,
    OldOut,
    Log,
    OldLog,
    BaseLog(String),
    Base(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(JsonSchema))]
pub enum ValgrindTool {
    Callgrind,
    Memcheck,
    Helgrind,
    DRD,
    Massif,
    DHAT,
    BBV,
}

pub trait Parser {
    type Output;

    fn parse(&self, output: &ToolOutputPath) -> Result<Self::Output>;
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
        tool_args.set_log_arg(output_path, config.outfile_modifier.as_ref());

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
            .and_then(|output| {
                check_exit(
                    self.tool,
                    &executable,
                    output,
                    &output_path.to_log_output(),
                    exit_with.as_ref(),
                )
            })?;

        Ok(ToolOutput {
            tool: self.tool,
            output,
        })
    }
}

impl ToolConfig {
    fn parse(
        &self,
        meta: &Metadata,
        log_path: &ToolOutputPath,
        out_path: Option<&ToolOutputPath>,
    ) -> Result<(ToolSummary, Vec<LogfileSummary>)> {
        let mut tool_summary = ToolSummary {
            tool: self.tool,
            log_paths: log_path.real_paths()?,
            out_paths: out_path.map_or_else(|| Ok(Vec::default()), ToolOutputPath::real_paths)?,
            summaries: vec![],
        };

        let parser: Box<dyn Parser<Output = Vec<LogfileSummary>>> =
            if let ValgrindTool::DHAT = self.tool {
                Box::new(DhatLogfileParser {
                    root_dir: meta.project_root.clone(),
                })
            } else {
                Box::new(LogfileParser {
                    root_dir: meta.project_root.clone(),
                })
            };

        let logfile_summaries = parser.as_ref().parse(log_path)?;

        for logfile_summary in &logfile_summaries {
            tool_summary.summaries.push(logfile_summary.into());
        }

        Ok((tool_summary, logfile_summaries))
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
        }
    }
}

impl ToolConfigs {
    pub fn has_tools_enabled(&self) -> bool {
        self.0.iter().any(|t| t.is_enabled)
    }

    pub fn output_paths(&self, output_path: &ToolOutputPath) -> Vec<ToolOutputPath> {
        self.0
            .iter()
            .filter(|t| t.is_enabled)
            .map(|t| output_path.to_tool_output(t.tool))
            .collect()
    }

    fn print_headline(meta: &Metadata, tool_config: &ToolConfig) {
        if meta.args.output_format == OutputFormat::Default {
            println!("{}", tool_headline(tool_config.tool));
        }
    }

    fn print(
        meta: &Metadata,
        tool_config: &ToolConfig,
        logfile_summaries: &[LogfileSummary],
        output_paths: &[PathBuf],
    ) -> Result<()> {
        if meta.args.output_format == OutputFormat::Default {
            for logfile_summary in logfile_summaries {
                LogfileSummaryFormatter::print(
                    logfile_summary,
                    tool_config.args.verbose,
                    logfile_summaries.len() > 1,
                    matches!(tool_config.tool, ValgrindTool::BBV),
                )?;
            }

            for path in output_paths
                .iter()
                .map(|p| make_relative(&meta.project_root, p))
            {
                println!(
                    "  {:<18}{}",
                    "Outfile:",
                    path.display().to_string().blue().bold()
                );
            }
        }
        Ok(())
    }

    pub fn parse(
        tool_config: &ToolConfig,
        meta: &Metadata,
        log_path: &ToolOutputPath,
        out_path: Option<&ToolOutputPath>,
    ) -> Result<(ToolSummary, Vec<LogfileSummary>)> {
        let mut tool_summary = ToolSummary {
            tool: tool_config.tool,
            log_paths: log_path.real_paths()?,
            out_paths: out_path.map_or_else(|| Ok(Vec::default()), ToolOutputPath::real_paths)?,
            summaries: vec![],
        };

        let parser: Box<dyn Parser<Output = Vec<LogfileSummary>>> =
            if let ValgrindTool::DHAT = tool_config.tool {
                Box::new(DhatLogfileParser {
                    root_dir: meta.project_root.clone(),
                })
            } else {
                Box::new(LogfileParser {
                    root_dir: meta.project_root.clone(),
                })
            };

        let logfile_summaries = parser.as_ref().parse(log_path)?;

        for logfile_summary in &logfile_summaries {
            tool_summary.summaries.push(logfile_summary.into());
        }

        Ok((tool_summary, logfile_summaries))
    }

    pub fn run_loaded_vs_base(
        &self,
        meta: &Metadata,
        output_path: &ToolOutputPath,
    ) -> Result<Vec<ToolSummary>> {
        let mut tool_summaries = vec![];
        for tool_config in self.0.iter().filter(|t| t.is_enabled) {
            let tool = tool_config.tool;

            let output_path = output_path.to_tool_output(tool);
            let log_path = output_path.to_log_output();

            Self::print_headline(meta, tool_config);

            let (tool_summary, logfile_summaries) = tool_config.parse(meta, &log_path, None)?;

            Self::print(
                meta,
                tool_config,
                &logfile_summaries,
                &tool_summary.out_paths,
            )?;

            log_path.dump_log(log::Level::Info, &mut stderr())?;

            tool_summaries.push(tool_summary);
        }

        Ok(tool_summaries)
    }

    pub fn run(
        &self,
        meta: &Metadata,
        executable: &Path,
        executable_args: &[OsString],
        options: &RunOptions,
        output_path: &ToolOutputPath,
    ) -> Result<Vec<ToolSummary>> {
        let mut tool_summaries = vec![];
        for tool_config in self.0.iter().filter(|t| t.is_enabled) {
            let tool = tool_config.tool;

            let command = ToolCommand::new(tool, meta);

            let output_path = output_path.to_tool_output(tool);
            let log_path = output_path.to_log_output();

            Self::print_headline(meta, tool_config);

            let output = command.run(
                tool_config.clone(),
                executable,
                executable_args,
                options.clone(),
                &output_path,
            )?;

            let (tool_summary, logfile_summaries) = Self::parse(
                tool_config,
                meta,
                &log_path,
                tool.has_output_file().then_some(&output_path),
            )?;

            Self::print(
                meta,
                tool_config,
                &logfile_summaries,
                &tool_summary.out_paths,
            )?;

            output.dump_log(log::Level::Info);
            log_path.dump_log(log::Level::Info, &mut stderr())?;

            tool_summaries.push(tool_summary);
        }

        Ok(tool_summaries)
    }
}

impl ToolOutput {
    pub fn dump_log(&self, log_level: Level) {
        if log::log_enabled!(log_level) {
            let (stdout, stderr) = (&self.output.stdout, &self.output.stderr);
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

impl ToolOutputPath {
    pub fn new(
        kind: ToolOutputPathKind,
        tool: ValgrindTool,
        baseline_kind: &BaselineKind,
        base_dir: &Path,
        module: &str,
        name: &str,
    ) -> Self {
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
        let sanitized_name = truncate_str_utf8(&sanitized_name, 200);
        Self {
            kind,
            tool,
            baseline_kind: baseline_kind.clone(),
            dir: current
                .join(base_dir)
                .join(module_path)
                .join(sanitized_name),
            name: sanitized_name.to_owned(),
            modifiers: vec![],
        }
    }

    /// Initialize and create the output directory and organize files
    ///
    /// This method moves the old output to `$TOOL_ID.*.out.old`
    pub fn with_init(
        kind: ToolOutputPathKind,
        tool: ValgrindTool,
        baseline_kind: &BaselineKind,
        base_dir: &Path,
        module: &str,
        name: &str,
    ) -> Result<Self> {
        let output = Self::new(kind, tool, baseline_kind, base_dir, module, name);
        output.init()?;
        Ok(output)
    }

    pub fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.dir).with_context(|| {
            format!(
                "Failed to create benchmark directory: '{}'",
                self.dir.display()
            )
        })
    }

    pub fn clear(&self) -> Result<()> {
        for entry in self.real_paths()? {
            std::fs::remove_file(&entry).with_context(|| {
                format!("Failed to remove benchmark file: '{}'", entry.display())
            })?;
        }
        Ok(())
    }

    pub fn shift(&self) -> Result<()> {
        match self.baseline_kind {
            BaselineKind::Old => {
                self.to_base_path().clear()?;
                for entry in self.real_paths()? {
                    let extension = entry.extension().expect("An extension should be present");
                    let mut extension = extension.to_owned();
                    extension.push(".old");
                    let new_path = entry.with_extension(extension);
                    std::fs::rename(&entry, &new_path).with_context(|| {
                        format!(
                            "Failed to move benchmark file from '{}' to '{}'",
                            entry.display(),
                            new_path.display()
                        )
                    })?;
                }
                Ok(())
            }
            BaselineKind::Name(_) => self.clear(),
        }
    }

    pub fn exists(&self) -> bool {
        self.real_paths().map_or(false, |p| !p.is_empty())
    }

    pub fn is_multiple(&self) -> bool {
        self.real_paths().map_or(false, |p| p.len() > 1)
    }

    pub fn to_base_path(&self) -> Self {
        Self {
            kind: match (&self.kind, &self.baseline_kind) {
                (ToolOutputPathKind::Out, BaselineKind::Old) => ToolOutputPathKind::OldOut,
                (ToolOutputPathKind::Out, BaselineKind::Name(name)) => {
                    ToolOutputPathKind::Base(name.to_string())
                }
                (ToolOutputPathKind::Log, BaselineKind::Old) => ToolOutputPathKind::OldLog,
                (ToolOutputPathKind::Log, BaselineKind::Name(name)) => {
                    ToolOutputPathKind::BaseLog(name.to_string())
                }
                (kind, _) => kind.clone(),
            },
            tool: self.tool,
            baseline_kind: self.baseline_kind.clone(),
            name: self.name.clone(),
            dir: self.dir.clone(),
            modifiers: self.modifiers.clone(),
        }
    }

    pub fn to_tool_output(&self, tool: ValgrindTool) -> Self {
        Self {
            tool,
            kind: self.kind.clone(),
            baseline_kind: self.baseline_kind.clone(),
            name: self.name.clone(),
            dir: self.dir.clone(),
            modifiers: self.modifiers.clone(),
        }
    }

    pub fn to_log_output(&self) -> Self {
        Self {
            kind: match &self.kind {
                ToolOutputPathKind::Out | ToolOutputPathKind::OldOut => ToolOutputPathKind::Log,
                ToolOutputPathKind::Base(name) => ToolOutputPathKind::BaseLog(name.clone()),
                kind => kind.clone(),
            },
            tool: self.tool,
            baseline_kind: self.baseline_kind.clone(),
            name: self.name.clone(),
            dir: self.dir.clone(),
            modifiers: self.modifiers.clone(),
        }
    }

    pub fn open(&self) -> Result<File> {
        let path = self.to_path();
        File::open(&path).with_context(|| {
            format!(
                "Error opening {} output file '{}'",
                self.tool.id(),
                path.display()
            )
        })
    }

    pub fn lines(&self) -> Result<impl Iterator<Item = String>> {
        let file = self.open()?;
        Ok(BufReader::new(file)
            .lines()
            .map(std::result::Result::unwrap))
    }

    pub fn dump_log(&self, log_level: log::Level, writer: &mut impl Write) -> Result<()> {
        if log_enabled!(log_level) {
            for path in self.real_paths()? {
                log::log!(
                    log_level,
                    "{} log output '{}':",
                    self.tool.id(),
                    path.display()
                );

                let file = File::open(&path).with_context(|| {
                    format!(
                        "Error opening {} output file '{}'",
                        self.tool.id(),
                        path.display()
                    )
                })?;

                let mut reader = BufReader::new(file);
                std::io::copy(&mut reader, writer)?;
            }
        }
        Ok(())
    }

    pub fn extension(&self) -> String {
        match (&self.kind, self.modifiers.is_empty()) {
            (ToolOutputPathKind::Out, true) => "out".to_owned(),
            (ToolOutputPathKind::Out, false) => format!("out.{}", self.modifiers.join(".")),
            (ToolOutputPathKind::Log, true) => "log".to_owned(),
            (ToolOutputPathKind::Log, false) => format!("log.{}", self.modifiers.join(".")),
            (ToolOutputPathKind::OldOut, true) => "out.old".to_owned(),
            (ToolOutputPathKind::OldOut, false) => format!("out.{}.old", self.modifiers.join(".")),
            (ToolOutputPathKind::OldLog, true) => "log.old".to_owned(),
            (ToolOutputPathKind::OldLog, false) => format!("log.{}.old", self.modifiers.join(".")),
            (ToolOutputPathKind::BaseLog(name), true) => {
                format!("log.base@{name}")
            }
            (ToolOutputPathKind::BaseLog(name), false) => {
                format!("log.{}.base@{name}", self.modifiers.join("."))
            }
            (ToolOutputPathKind::Base(name), true) => format!("out.base@{name}"),
            (ToolOutputPathKind::Base(name), false) => {
                format!("out.{}.base@{name}", self.modifiers.join("."))
            }
        }
    }

    pub fn with_modifiers<I, T>(&self, modifiers: T) -> Self
    where
        I: Into<String>,
        T: IntoIterator<Item = I>,
    {
        Self {
            kind: self.kind.clone(),
            tool: self.tool,
            baseline_kind: self.baseline_kind.clone(),
            dir: self.dir.clone(),
            name: self.name.clone(),
            modifiers: modifiers.into_iter().map(Into::into).collect(),
        }
    }

    pub fn to_path(&self) -> PathBuf {
        self.dir.join(format!(
            "{}.{}.{}",
            self.tool.id(),
            self.name,
            self.extension()
        ))
    }

    pub fn real_paths(&self) -> Result<Vec<PathBuf>> {
        let mut paths = vec![];
        for entry in std::fs::read_dir(&self.dir).with_context(|| {
            format!(
                "Failed opening benchmark directory: '{}'",
                self.dir.display()
            )
        })? {
            let path = entry?;
            let file_name = path.file_name().to_string_lossy().to_string();
            if let Some(suffix) =
                file_name.strip_prefix(format!("{}.{}.", self.tool.id(), self.name).as_str())
            {
                #[allow(clippy::case_sensitive_file_extension_comparisons)]
                let is_match = match &self.kind {
                    ToolOutputPathKind::Out => {
                        suffix.starts_with("out")
                            && !(suffix.ends_with(".old")
                                || suffix
                                    .rsplit_once('.')
                                    .map_or(false, |(_, b)| b.starts_with("base@")))
                    }
                    ToolOutputPathKind::Log => {
                        suffix.starts_with("log")
                            && !(suffix.ends_with(".old")
                                || suffix
                                    .rsplit_once('.')
                                    .map_or(false, |(_, b)| b.starts_with("base@")))
                    }
                    ToolOutputPathKind::OldOut => {
                        suffix.starts_with("out") && suffix.ends_with(".old")
                    }
                    ToolOutputPathKind::OldLog => {
                        suffix.starts_with("log") && suffix.ends_with(".old")
                    }
                    ToolOutputPathKind::BaseLog(name) => {
                        suffix.starts_with("log")
                            && suffix.ends_with(format!(".base@{name}").as_str())
                    }
                    ToolOutputPathKind::Base(name) => {
                        suffix.starts_with("out")
                            && suffix.ends_with(format!(".base@{name}").as_str())
                    }
                };

                if is_match {
                    paths.push(path.path());
                }
            }
        }
        Ok(paths)
    }
}

impl Display for ToolOutputPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.to_path().display()))
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

impl TryFrom<&str> for ValgrindTool {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "dhat" => Ok(ValgrindTool::DHAT),
            "callgrind" => Ok(ValgrindTool::Callgrind),
            "memcheck" => Ok(ValgrindTool::Memcheck),
            "helgrind" => Ok(ValgrindTool::Helgrind),
            "drd" => Ok(ValgrindTool::DRD),
            "massif" => Ok(ValgrindTool::Massif),
            "exp-bbv" => Ok(ValgrindTool::BBV),
            v => Err(anyhow!("Unknown tool '{}'", v)),
        }
    }
}

pub fn check_exit(
    tool: ValgrindTool,
    executable: &Path,
    output: Output,
    output_path: &ToolOutputPath,
    exit_with: Option<&ExitWith>,
) -> Result<Output> {
    let Some(status_code) = output.status.code() else {
        return Err(Error::ProcessError((tool.id(), output, Some(output_path.clone()))).into());
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
            Err(Error::ProcessError((tool.id(), output, Some(output_path.clone()))).into())
        }
        (0i32, Some(ExitWith::Failure)) => {
            error!(
                "{}: Expected '{}' to fail but it succeeded",
                tool.id(),
                executable.display(),
            );
            Err(Error::ProcessError((tool.id(), output, Some(output_path.clone()))).into())
        }
        (_, Some(ExitWith::Failure)) => Ok(output),
        (code, Some(ExitWith::Success)) => {
            error!(
                "{}: Expected '{}' to succeed but it terminated with '{}'",
                tool.id(),
                executable.display(),
                code
            );
            Err(Error::ProcessError((tool.id(), output, Some(output_path.clone()))).into())
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
            Err(Error::ProcessError((tool.id(), output, Some(output_path.clone()))).into())
        }
        _ => Err(Error::ProcessError((tool.id(), output, Some(output_path.clone()))).into()),
    }
}
