// spell-checker: ignore extbase extbasename extold
pub mod args;
pub mod error_metric_parser;
pub mod generic_parser;
pub mod logfile_parser;

use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt::{Display, Write as FmtWrite};
use std::fs::{DirEntry, File};
use std::io::{stderr, BufRead, BufReader, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output};

use anyhow::{anyhow, Context, Result};
use lazy_static::lazy_static;
use log::{debug, error, log_enabled};
use logfile_parser::Logfile;
use regex::Regex;
#[cfg(feature = "schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use self::args::ToolArgs;
use super::args::NoCapture;
use super::bin_bench::Delay;
use super::callgrind::parser::{parse_header, CallgrindProperties};
use super::common::{Assistant, Config, ModulePath, Sandbox};
use super::format::{
    print_no_capture_footer, tool_headline, Formatter, OutputFormat, VerticalFormat,
};
use super::meta::Metadata;
use super::summary::{BaselineKind, ToolRunSummaries, ToolSummary};
use crate::api::{self, ExitWith, Stream};
use crate::error::Error;
use crate::util::{self, ilog10, resolve_binary_path, truncate_str_utf8, EitherOrBoth};

lazy_static! {
    // This regex matches the original file name as it is created by callgrind. The baseline <name>
    // (base@<name>) can only consist of ascii and underscore characters. Flamegraph files are
    // ignored by this regex
    static ref CALLGRIND_ORIG_FILENAME_RE: Regex = Regex::new(
        r"^(?<tool>callgrind)(?<name>[.][^.]+?([.][^0-9][^.]*?)?)(?<pid>[.][0-9]+)?(?<type>[.](out|log))(?<base>[.](old|base@[^.]+))?(?<part>[.][0-9]+)?(?<thread>-[0-9]+)?$"
    )
    .expect("Regex should compile");
    static ref BBV_ORIG_FILENAME_RE: Regex = Regex::new(
        r"^(?<tool>exp-bbv)(?<name>[.][^.]+?([.][^0-9][^.]*?)?)(?<pid>[.][0-9]+)?(?<kind>[.](bb|pc))?(?<type>[.](out|log))(?<base>[.](old|base@[^.]+))?(?<thread>[.][0-9]+)?$"
    )
    .expect("Regex should compile");
}

#[derive(Debug, Default, Clone)]
pub struct RunOptions {
    pub env_clear: bool,
    pub current_dir: Option<PathBuf>,
    pub exit_with: Option<ExitWith>,
    pub envs: Vec<(OsString, OsString)>,
    pub stdin: Option<api::Stdin>,
    pub stdout: Option<api::Stdio>,
    pub stderr: Option<api::Stdio>,
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
    nocapture: NoCapture,
    command: Command,
}

pub struct ToolOutput {
    pub tool: ValgrindTool,
    pub output: Option<Output>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolOutputPath {
    pub kind: ToolOutputPathKind,
    pub tool: ValgrindTool,
    pub baseline_kind: BaselineKind,
    /// The final directory of all the output files
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

impl ToolCommand {
    pub fn new(tool: ValgrindTool, meta: &Metadata, nocapture: NoCapture) -> Self {
        Self {
            tool,
            nocapture,
            command: meta.into(),
        }
    }

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
        tool_args.set_output_arg(output_path, config.outfile_modifier.as_ref());
        tool_args.set_log_arg(output_path, config.outfile_modifier.as_ref());

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

        if self.tool == ValgrindTool::Callgrind {
            debug!("Applying --nocapture options");
            self.nocapture.apply(&mut self.command);
        }

        if let Some(stdin) = stdin {
            stdin
                .apply(&mut self.command, Stream::Stdin, child.as_mut())
                .map_err(|error| {
                    Error::BenchmarkError(ValgrindTool::Callgrind, module_path.clone(), error)
                })?;
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
            NoCapture::True | NoCapture::Stderr | NoCapture::Stdout
                if self.tool == ValgrindTool::Callgrind =>
            {
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
                return Err(Error::ProcessError((
                    module_path.join("setup").to_string(),
                    None,
                    status,
                    None,
                ))
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

impl ToolConfig {
    pub fn new<T>(tool: ValgrindTool, is_enabled: bool, args: T, modifier: Option<String>) -> Self
    where
        T: Into<ToolArgs>,
    {
        Self {
            tool,
            is_enabled,
            args: args.into(),
            outfile_modifier: modifier,
        }
    }

    fn parse_load(
        &self,
        meta: &Metadata,
        log_path: &ToolOutputPath,
        out_path: Option<&ToolOutputPath>,
    ) -> Result<ToolSummary> {
        let parser = logfile_parser::parser_factory(self.tool, meta.project_root.clone());

        // TODO: Use load-baseline as the new dataset instead of the old dataset. Not only here but
        // in other places, too
        let parsed_new = parser.parse(log_path)?;
        let parsed_old = parser.parse(&log_path.to_base_path())?;

        match (parsed_new.is_empty(), parsed_old.is_empty()) {
            (true, true) => Err(anyhow!(
                "The baselines '{}' and '{}' don't exist",
                &meta.args.baseline.as_ref().unwrap(),
                &meta.args.load_baseline.as_ref().unwrap()
            )),
            (true, false) => Err(anyhow!(
                "The baseline '{}' doesn't exist",
                &meta.args.baseline.as_ref().unwrap(),
            )),
            (false, true) => Err(anyhow!(
                "The loaded baseline '{}' doesn't exist",
                &meta.args.load_baseline.as_ref().unwrap()
            )),
            (false, false) => {
                let summaries = ToolRunSummaries::from(EitherOrBoth::Both(parsed_new, parsed_old));
                Ok(ToolSummary {
                    tool: self.tool,
                    log_paths: log_path.real_paths()?,
                    out_paths: out_path
                        .map_or_else(|| Ok(Vec::default()), ToolOutputPath::real_paths)?,
                    summaries,
                })
            }
        }
    }
}

impl From<api::Tool> for ToolConfig {
    fn from(value: api::Tool) -> Self {
        let tool = value.kind.into();
        Self {
            tool,
            is_enabled: value.enable.unwrap_or(true),
            args: ToolArgs::from_raw_args(tool, value.raw_args),
            outfile_modifier: None,
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

    fn print_headline(tool_config: &ToolConfig, output_format: &OutputFormat) {
        if output_format.is_default() {
            println!("{}", tool_headline(tool_config.tool));
        }
    }

    fn print(
        meta: &Metadata,
        output_format: &OutputFormat,
        tool_run_summaries: &ToolRunSummaries,
    ) -> Result<()> {
        VerticalFormat.print(meta, output_format, (None, None), tool_run_summaries)
    }

    pub fn parse(
        tool_config: &ToolConfig,
        meta: &Metadata,
        log_path: &ToolOutputPath,
        out_path: Option<&ToolOutputPath>,
        old_summaries: Vec<Logfile>,
    ) -> Result<ToolSummary> {
        let parser = logfile_parser::parser_factory(tool_config.tool, meta.project_root.clone());

        let parsed_new = parser.parse(log_path)?;

        let summaries = match (parsed_new.is_empty(), old_summaries.is_empty()) {
            (true, false | true) => return Err(anyhow!("A new dataset should always be present")),
            (false, true) => ToolRunSummaries::from(EitherOrBoth::Left(parsed_new)),
            (false, false) => ToolRunSummaries::from(EitherOrBoth::Both(parsed_new, old_summaries)),
        };

        Ok(ToolSummary {
            tool: tool_config.tool,
            log_paths: log_path.real_paths()?,
            out_paths: out_path.map_or_else(|| Ok(Vec::default()), ToolOutputPath::real_paths)?,
            summaries,
        })
    }

    pub fn run_loaded_vs_base(
        &self,
        meta: &Metadata,
        output_path: &ToolOutputPath,
        output_format: &OutputFormat,
    ) -> Result<Vec<ToolSummary>> {
        let mut tool_summaries = vec![];
        for tool_config in self.0.iter().filter(|t| t.is_enabled) {
            Self::print_headline(tool_config, output_format);

            let tool = tool_config.tool;

            let output_path = output_path.to_tool_output(tool);
            let log_path = output_path.to_log_output();

            let tool_summary = tool_config.parse_load(meta, &log_path, None)?;

            Self::print(meta, output_format, &tool_summary.summaries)?;

            log_path.dump_log(log::Level::Info, &mut stderr())?;

            tool_summaries.push(tool_summary);
        }

        Ok(tool_summaries)
    }

    pub fn run(
        &self,
        config: &Config,
        executable: &Path,
        executable_args: &[OsString],
        run_options: &RunOptions,
        output_path: &ToolOutputPath,
        save_baseline: bool,
        module_path: &ModulePath,
        sandbox: Option<&api::Sandbox>,
        setup: Option<&Assistant>,
        teardown: Option<&Assistant>,
        delay: Option<&Delay>,
        output_format: &OutputFormat,
    ) -> Result<Vec<ToolSummary>> {
        let mut tool_summaries = vec![];
        for tool_config in self.0.iter().filter(|t| t.is_enabled) {
            // Print the headline as soon as possible, so if there are any errors, the errors shown
            // in the terminal output can be associated with the tool
            Self::print_headline(tool_config, output_format);

            let tool = tool_config.tool;

            let command = ToolCommand::new(tool, &config.meta, NoCapture::False);

            let output_path = output_path.to_tool_output(tool);
            let log_path = output_path.to_log_output();

            let parser = logfile_parser::parser_factory(tool, config.meta.project_root.clone());

            let old_summaries = parser.parse(&log_path.to_base_path())?;
            if save_baseline {
                output_path.clear()?;
                log_path.clear()?;
            }

            let sandbox = sandbox
                .map(|sandbox| Sandbox::setup(sandbox, &config.meta))
                .transpose()?;

            let mut child = setup
                .as_ref()
                .map_or(Ok(None), |setup| setup.run(config, module_path))?;

            if let Some(delay) = delay {
                if let Err(error) = delay.run() {
                    if let Some(mut child) = child.take() {
                        // To avoid zombies
                        child.kill()?;
                        return Err(error);
                    }
                }
            }

            let output = command.run(
                tool_config.clone(),
                executable,
                executable_args,
                run_options.clone(),
                &output_path,
                module_path,
                child,
            )?;

            if let Some(teardown) = teardown {
                teardown.run(config, module_path)?;
            }

            print_no_capture_footer(
                NoCapture::False,
                run_options.stdout.as_ref(),
                run_options.stderr.as_ref(),
            );

            if let Some(sandbox) = sandbox {
                sandbox.reset()?;
            }

            let tool_summary = Self::parse(
                tool_config,
                &config.meta,
                &log_path,
                tool.has_output_file().then_some(&output_path),
                old_summaries,
            )?;

            Self::print(&config.meta, output_format, &tool_summary.summaries)?;

            output.dump_log(log::Level::Info);
            log_path.dump_log(log::Level::Info, &mut stderr())?;

            tool_summaries.push(tool_summary);
        }

        Ok(tool_summaries)
    }
}

impl ToolOutput {
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

impl ToolOutputPath {
    /// Create a new `ToolOutputPath`.
    ///
    /// The `base_dir` is supposed to be the same as [`crate::runner::meta::Metadata::target_dir`].
    pub fn new(
        kind: ToolOutputPathKind,
        tool: ValgrindTool,
        baseline_kind: &BaselineKind,
        base_dir: &Path,
        module: &ModulePath,
        name: &str,
    ) -> Self {
        let current = base_dir;
        let module_path: PathBuf = module.to_string().split("::").collect();
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
        let output = Self::new(
            kind,
            tool,
            baseline_kind,
            base_dir,
            &ModulePath::new(module),
            name,
        );
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
                (
                    ToolOutputPathKind::Out | ToolOutputPathKind::Base(_),
                    BaselineKind::Name(name),
                ) => ToolOutputPathKind::Base(name.to_string()),
                (ToolOutputPathKind::Log, BaselineKind::Old) => ToolOutputPathKind::OldLog,
                (
                    ToolOutputPathKind::Log | ToolOutputPathKind::BaseLog(_),
                    BaselineKind::Name(name),
                ) => ToolOutputPathKind::BaseLog(name.to_string()),
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
            (ToolOutputPathKind::Out, false) => format!("{}.out", self.modifiers.join(".")),
            (ToolOutputPathKind::Log, true) => "log".to_owned(),
            (ToolOutputPathKind::Log, false) => format!("{}.log", self.modifiers.join(".")),
            (ToolOutputPathKind::OldOut, true) => "out.old".to_owned(),
            (ToolOutputPathKind::OldOut, false) => format!("{}.out.old", self.modifiers.join(".")),
            (ToolOutputPathKind::OldLog, true) => "log.old".to_owned(),
            (ToolOutputPathKind::OldLog, false) => format!("{}.log.old", self.modifiers.join(".")),
            (ToolOutputPathKind::BaseLog(name), true) => {
                format!("log.base@{name}")
            }
            (ToolOutputPathKind::BaseLog(name), false) => {
                format!("{}.log.base@{name}", self.modifiers.join("."))
            }
            (ToolOutputPathKind::Base(name), true) => format!("out.base@{name}"),
            (ToolOutputPathKind::Base(name), false) => {
                format!("{}.out.base@{name}", self.modifiers.join("."))
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

    // TODO: RENAME to something less confusing and return String
    pub fn to_path(&self) -> PathBuf {
        self.dir.join(format!(
            "{}.{}.{}",
            self.tool.id(),
            self.name,
            self.extension()
        ))
    }

    /// Walk the benchmark directory (non-recursive)
    pub fn walk_dir(&self) -> Result<impl Iterator<Item = DirEntry>> {
        std::fs::read_dir(&self.dir)
            .with_context(|| {
                format!(
                    "Failed opening benchmark directory: '{}'",
                    self.dir.display()
                )
            })
            .map(|i| i.into_iter().filter_map(Result::ok))
    }

    /// Strip the `<tool>.<name>` prefix from a `file_name`
    pub fn strip_prefix<'a>(&self, file_name: &'a str) -> Option<&'a str> {
        file_name.strip_prefix(format!("{}.{}", self.tool.id(), self.name).as_str())
    }

    pub fn get_prefix(&self) -> String {
        format!("{}.{}", self.tool.id(), self.name)
    }

    /// Return the `real` paths of a tool's output files
    ///
    /// A tool can have many output files so [`Self::to_path`] is not enough
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    pub fn real_paths(&self) -> Result<Vec<PathBuf>> {
        let mut paths = vec![];
        for entry in self.walk_dir()? {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            // We can silently ignore all paths which don't follow this scheme, for example
            // (`summary.json`)
            if let Some(suffix) = self.strip_prefix(&file_name) {
                let is_match = match &self.kind {
                    ToolOutputPathKind::Out => suffix.ends_with(".out"),
                    ToolOutputPathKind::Log => suffix.ends_with(".log"),
                    ToolOutputPathKind::OldOut => suffix.ends_with(".out.old"),
                    ToolOutputPathKind::OldLog => suffix.ends_with(".log.old"),
                    ToolOutputPathKind::BaseLog(name) => {
                        suffix.ends_with(format!(".log.base@{name}").as_str())
                    }
                    ToolOutputPathKind::Base(name) => {
                        suffix.ends_with(format!(".out.base@{name}").as_str())
                    }
                };

                if is_match {
                    paths.push(entry.path());
                }
            }
        }
        Ok(paths)
    }

    // TODO: CLEANUP?
    pub fn real_paths_with_modifier(&self) -> Result<Vec<(PathBuf, String)>> {
        let mut paths = vec![];
        for entry in self.walk_dir()? {
            let file_name = entry.file_name().to_string_lossy().to_string();

            // We can silently ignore all paths which don't follow this scheme, for example
            // (`summary.json`)
            if let Some(suffix) = self.strip_prefix(&file_name) {
                let modifiers = match &self.kind {
                    ToolOutputPathKind::Out => suffix.strip_suffix(".out"),
                    ToolOutputPathKind::Log => suffix.strip_suffix(".log"),
                    ToolOutputPathKind::OldOut => suffix.strip_suffix(".out.old"),
                    ToolOutputPathKind::OldLog => suffix.strip_suffix(".log.old"),
                    ToolOutputPathKind::BaseLog(name) => {
                        suffix.strip_suffix(format!(".log.base@{name}").as_str())
                    }
                    ToolOutputPathKind::Base(name) => {
                        suffix.strip_suffix(format!(".out.base@{name}").as_str())
                    }
                };

                if let Some(modifiers) = modifiers {
                    paths.push((entry.path(), modifiers.to_owned()));
                }
                // TODO: Return empty string for modifier?
            }
        }
        Ok(paths)
    }

    /// Sanitize callgrind output file names
    ///
    /// This method will remove empty files which are occasionally produced by callgrind and only
    /// cause problems in the parser. The files are renamed from the callgrind file naming scheme to
    /// ours which is easier to handle.
    ///
    /// The information about pids, parts and threads is obtained by parsing the header from the
    /// callgrind output files instead of relying on the sometimes flaky file names produced by
    /// `callgrind`. The header is around 10-20 lines, so this method should be still sufficiently
    /// fast. Additionally, `callgrind` might change the naming scheme of its' files, so using the
    /// headers makes us more independent from a specific valgrind/callgrind version.
    pub fn sanitize_callgrind(&self) -> Result<()> {
        type Grouped<'a> = (PathBuf, Option<String>, String, CallgrindProperties);

        // To figure out if there are multiple pids/parts/threads present, it's necessary to group
        // the files in this map. The order doesn't matter since we only rename the original file
        // names, which doesn't need to follow a specific order.
        //
        // At first we group by pid and then by part in different hashmaps. The threads are grouped
        // in a vector.
        let mut groups: HashMap<Option<i32>, HashMap<Option<u64>, Vec<Grouped>>> = HashMap::new();

        for entry in self.walk_dir()? {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            if let Some(caps) = CALLGRIND_ORIG_FILENAME_RE.captures(&file_name) {
                // Callgrind sometimes creates empty files for no reason. We clean them
                // up here
                if entry.metadata()?.size() == 0 {
                    std::fs::remove_file(entry.path())?;
                    continue;
                }

                if caps.name("type").unwrap().as_str() == ".out" {
                    // We don't sanitize old files. It's not needed if the new files are always
                    // sanitized. However, we do sanitize `base@<name>` file names.
                    let base = if let Some(base) = caps.name("base") {
                        if base.as_str() == ".old" {
                            continue;
                        }

                        Some(base.as_str().to_owned())
                    } else {
                        None
                    };

                    let properties = parse_header(
                        &mut BufReader::new(File::open(entry.path())?)
                            .lines()
                            .map(Result::unwrap),
                    )?;

                    let name = caps
                        .name("name")
                        .expect("The name should always be present")
                        .as_str();

                    if let Some(pids) = groups.get_mut(&properties.pid) {
                        if let Some(parts) = pids.get_mut(&properties.part) {
                            parts.push((entry.path(), base, name.to_owned(), properties));
                        } else {
                            pids.insert(
                                properties.part,
                                vec![(entry.path(), base, name.to_owned(), properties)],
                            );
                        }
                    } else {
                        groups.insert(
                            properties.pid,
                            HashMap::from([(
                                properties.part,
                                vec![(entry.path(), base, name.to_owned(), properties)],
                            )]),
                        );
                    }
                }
            }
        }

        let multiple_pids = groups.len() > 1;
        for parts in groups.values() {
            let multiple_parts = parts.len() > 1;
            for threads in parts.values() {
                let multiple_threads = threads.len() > 1;
                for (orig_path, base, name, properties) in threads {
                    let mut new_file_name = format!("callgrind{name}");

                    if multiple_pids {
                        if let Some(pid) = properties.pid {
                            write!(new_file_name, ".{pid}").unwrap();
                        }
                    }
                    if multiple_parts {
                        if let Some(part) = properties.part {
                            let width = usize::try_from(ilog10(parts.len() as u64)).expect(
                                "The logarithm of the parts length should fit into a usize",
                            ) + 1;
                            write!(new_file_name, ".p{part:0width$}").unwrap();
                        }

                        // Integrate the thread number into the filename independently of the amount
                        // of threads
                        if let Some(thread) = properties.thread {
                            let width = usize::try_from(ilog10(threads.len() as u64)).expect(
                                "The logarithm of the threads length should fit into a usize",
                            ) + 1;
                            write!(new_file_name, ".t{thread:0width$}").unwrap();
                        }
                    } else if multiple_threads {
                        if let Some(thread) = properties.thread {
                            let width = usize::try_from(ilog10(threads.len() as u64)).expect(
                                "The logarithm of the threads length should fit into a usize",
                            ) + 1;
                            write!(new_file_name, ".t{thread:0width$}").unwrap();
                        }
                    } else {
                        // don't integrate parts or threads into the filename
                    }

                    new_file_name.push_str(".out");
                    if let Some(base) = base {
                        new_file_name.push_str(base);
                    }

                    let from = orig_path;
                    let to = from.with_file_name(new_file_name);

                    std::fs::rename(from, to)?;
                }
            }
        }

        Ok(())
    }

    // Sanitize bbv files
    //
    // The original output files of bb have a `.<number>` suffix if there are multiple threads. We
    // need the threads as `t<number>` in the modifier part of the final file names.
    //
    // For example: (orig -> sanitized)
    //
    // If there are multiple threads, the bb output file name doesn't include the first thread:
    //
    // `exp-bbv.bench_thread_in_subprocess.548365.bb.out` ->
    // `exp-bbv.bench_thread_in_subprocess.548365.t1.bb.out`
    //
    // `exp-bbv.bench_thread_in_subprocess.548365.bb.out.2` ->
    // `exp-bbv.bench_thread_in_subprocess.548365.t2.bb.out`
    pub fn sanitize_bbv(&self) -> Result<()> {
        // path, name, kind, base, thread,
        type Grouped<'a> = (PathBuf, String, String, Option<String>, String);

        let mut groups: HashMap<Option<String>, Vec<Grouped>> = HashMap::new();
        for entry in self.walk_dir()? {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            if let Some(caps) = BBV_ORIG_FILENAME_RE.captures(&file_name) {
                // Callgrind sometimes creates empty files for no reason. We clean them
                // up here
                if entry.metadata()?.size() == 0 {
                    std::fs::remove_file(entry.path())?;
                    continue;
                }

                // Log file names can stay the same. They don't include any information about
                // threads in their file name.
                if caps.name("type").unwrap().as_str() == ".out" {
                    // Don't sanitize old files.
                    let base = if let Some(base) = caps.name("base") {
                        if base.as_str() == ".old" {
                            continue;
                        }

                        Some(base.as_str().to_owned())
                    } else {
                        None
                    };

                    let name = caps
                        .name("name")
                        .expect("The name should always be present")
                        .as_str();

                    let kind = caps
                        .name("kind")
                        .expect("The kind (pc|bb) should be present")
                        .as_str();

                    let pid = caps.name("pid").map(|p| p.as_str().to_owned());

                    let thread = caps
                        .name("thread")
                        .map_or_else(|| ".1".to_owned(), |t| t.as_str().to_owned());

                    if let Some(threads) = groups.get_mut(&pid) {
                        threads.push((
                            entry.path(),
                            name.to_owned(),
                            kind.to_owned(),
                            base,
                            thread,
                        ));
                    } else {
                        groups.insert(
                            pid,
                            vec![(entry.path(), name.to_owned(), kind.to_owned(), base, thread)],
                        );
                    };
                }
            }
        }

        let multiple_pids = groups.len() > 1;
        for (pid, threads) in groups {
            let multiple_threads = threads.len() > 1;

            for (orig_path, name, kind, base, thread) in &threads {
                let mut new_file_name = format!("exp-bbv{name}");

                if multiple_pids {
                    if let Some(pid) = pid.as_ref() {
                        write!(new_file_name, "{pid}").unwrap();
                    }
                }

                if (multiple_pids || multiple_threads) && kind == ".bb" {
                    let width = usize::try_from(ilog10(threads.len() as u64))
                        .expect("The logarithm of the threads length should fit into a usize")
                        + 1;
                    let thread = thread
                        .strip_prefix('.')
                        .expect("The thread point prefix should be present");
                    write!(new_file_name, ".t{thread:0width$}").unwrap();
                }

                new_file_name.push_str(kind);
                new_file_name.push_str(".out");

                if let Some(base) = base {
                    new_file_name.push_str(base);
                }

                let from = orig_path;
                let to = from.with_file_name(new_file_name);

                std::fs::rename(from, to)?;
            }
        }

        Ok(())
    }

    /// Sanitize file names for a specific tool
    ///
    /// Empty files are cleaned up. For more details on a specific tool see the respective
    /// sanitize_<tool> method.
    pub fn sanitize(&self) -> Result<()> {
        match self.tool {
            ValgrindTool::Callgrind => self.sanitize_callgrind()?,
            ValgrindTool::BBV => self.sanitize_bbv()?,
            _ => {
                for entry in self.walk_dir()? {
                    if entry.metadata()?.size() == 0 {
                        std::fs::remove_file(entry.path())?;
                    }
                }
            }
        };

        Ok(())
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

impl Display for ValgrindTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.id())
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
    output: Option<Output>,
    status: ExitStatus,
    output_path: &ToolOutputPath,
    exit_with: Option<&ExitWith>,
) -> Result<Option<Output>> {
    let Some(status_code) = status.code() else {
        return Err(
            Error::ProcessError((tool.id(), output, status, Some(output_path.clone()))).into(),
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
            Err(Error::ProcessError((tool.id(), output, status, Some(output_path.clone()))).into())
        }
        (0i32, Some(ExitWith::Failure)) => {
            error!(
                "{}: Expected '{}' to fail but it succeeded",
                tool.id(),
                executable.display(),
            );
            Err(Error::ProcessError((tool.id(), output, status, Some(output_path.clone()))).into())
        }
        (_, Some(ExitWith::Failure)) => Ok(output),
        (code, Some(ExitWith::Success)) => {
            error!(
                "{}: Expected '{}' to succeed but it terminated with '{}'",
                tool.id(),
                executable.display(),
                code
            );
            Err(Error::ProcessError((tool.id(), output, status, Some(output_path.clone()))).into())
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
            Err(Error::ProcessError((tool.id(), output, status, Some(output_path.clone()))).into())
        }
        _ => {
            Err(Error::ProcessError((tool.id(), output, status, Some(output_path.clone()))).into())
        }
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::out("callgrind.some.name.out")]
    #[case::out_with_pid("callgrind.some.name.1234.out")]
    #[case::out_with_part("callgrind.some.name.out.1")]
    #[case::out_with_thread("callgrind.some.name.out-01")]
    #[case::out_with_part_and_thread("callgrind.some.name.out.1-01")]
    #[case::out_old("callgrind.some.name.out.old")]
    #[case::out_old_with_pid("callgrind.some.name.1234.out.old")]
    #[case::out_old_with_part("callgrind.some.name.out.old.1")]
    #[case::out_old_with_thread("callgrind.some.name.out.old-01")]
    #[case::out_old_with_part_and_thread("callgrind.some.name.out.old.1-01")]
    #[case::out_base("callgrind.some.name.out.base@default")]
    #[case::out_base_with_pid("callgrind.some.name.1234.out.base@default")]
    #[case::out_base_with_part("callgrind.some.name.out.base@default.1")]
    #[case::out_base_with_thread("callgrind.some.name.out.base@default-01")]
    #[case::out_base_with_part_and_thread("callgrind.some.name.out.base@default.1-01")]
    #[case::log("callgrind.some.name.log")]
    #[case::log_with_pid("callgrind.some.name.1234.log.1")]
    #[case::log_base("callgrind.some.name.log.base@default")]
    #[case::log_base_with_pid("callgrind.some.name.1234.log.base@default.1")]
    #[case::without_id("callgrind.bench_bubble_sort_allocate.out")]
    #[case::without_id_but_pid("callgrind.bench_bubble_sort_allocate.1234.out")]
    #[case::without_id_but_thread("callgrind.bench_bubble_sort_allocate.out-01")]
    #[case::without_id_but_part("callgrind.bench_bubble_sort_allocate.out.1")]
    fn test_callgrind_filename_regex(#[case] haystack: &str) {
        assert!(CALLGRIND_ORIG_FILENAME_RE.is_match(haystack));
    }

    #[rstest]
    #[case::bb_out("exp-bbv.some.name.bb.out")]
    #[case::bb_out_with_pid("exp-bbv.some.name.1234.bb.out")]
    #[case::bb_out_with_pid_and_thread("exp-bbv.some.name.1234.bb.out.1")]
    #[case::bb_out_with_thread("exp-bbv.some.name.bb.out.1")]
    #[case::pc_out("exp-bbv.some.name.pc.out")]
    #[case::log("exp-bbv.some.name.log")]
    #[case::log_with_pid("exp-bbv.some.name.1234.log")]
    fn test_bbv_filename_regex(#[case] haystack: &str) {
        assert!(BBV_ORIG_FILENAME_RE.is_match(haystack));
    }
}
