// spell-checker: ignore extbase extbasename extold
pub mod args;
pub mod error_metric_parser;
pub mod generic_parser;
pub mod logfile_parser;
pub mod parser;
pub mod regression;

use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt::{Display, Write as FmtWrite};
use std::fs::{DirEntry, File};
use std::io::{stderr, BufRead, BufReader, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output};

use anyhow::{anyhow, Context, Result};
use error_metric_parser::ErrorMetricLogfileParser;
use generic_parser::GenericLogfileParser;
use indexmap::IndexSet;
use lazy_static::lazy_static;
use log::{debug, error, log_enabled};
use parser::{Parser, ParserOutput};
use regex::Regex;
use regression::RegressionConfig;

use self::args::ToolArgs;
use super::args::NoCapture;
use super::bin_bench::Delay;
use super::cachegrind::regression::CachegrindRegressionConfig;
use super::callgrind::flamegraph::{
    BaselineFlamegraphGenerator, Config as FlamegraphConfig, Flamegraph, FlamegraphGenerator,
    LoadBaselineFlamegraphGenerator, SaveBaselineFlamegraphGenerator,
};
use super::callgrind::parser::{parse_header, Sentinel};
use super::callgrind::regression::CallgrindRegressionConfig;
use super::common::{Assistant, Baselines, Config, ModulePath, Sandbox};
use super::dhat::json_parser::JsonParser;
use super::dhat::logfile_parser::DhatLogfileParser;
use super::dhat::regression::DhatRegressionConfig;
use super::format::{print_no_capture_footer, Formatter, OutputFormat, VerticalFormatter};
use super::meta::Metadata;
use super::summary::{
    BaselineKind, BaselineName, BenchmarkSummary, Profile, ProfileData, ProfileTotal,
    ToolMetricSummary, ToolRegression,
};
use super::{cachegrind, callgrind, DEFAULT_TOGGLE};
use crate::api::{
    self, EntryPoint, ExitWith, RawArgs, Stream, Tool, ToolOutputFormat, Tools, ValgrindTool,
};
use crate::error::Error;
use crate::util::{self, resolve_binary_path, truncate_str_utf8, Glob};

lazy_static! {
    // This regex matches the original file name without the prefix as it is created by callgrind.
    // The baseline <name> (base@<name>) can only consist of ascii and underscore characters.
    // Flamegraph files are ignored by this regex
    static ref CALLGRIND_ORIG_FILENAME_RE: Regex = Regex::new(
        r"^(?<type>[.](out|log))(?<base>[.](old|base@[^.-]+))?(?<pid>[.][#][0-9]+)?(?<part>[.][0-9]+)?(?<thread>-[0-9]+)?$"
    )
    .expect("Regex should compile");

    /// This regex matches the original file name without the prefix as it is created by bbv
    static ref BBV_ORIG_FILENAME_RE: Regex = Regex::new(
        r"^(?<type>[.](?:out|log))(?<base>[.](old|base@[^.]+))?(?<bbv_type>[.](?:bb|pc))?(?<pid>[.][#][0-9]+)?(?<thread>[.][0-9]+)?$"
    )
    .expect("Regex should compile");

    /// This regex matches the original file name without the prefix as it is created by all tools
    /// other than callgrind and bbv.
    static ref GENERIC_ORIG_FILENAME_RE: Regex = Regex::new(
        r"^(?<type>[.](?:out|log))(?<base>[.](old|base@[^.]+))?(?<pid>[.][#][0-9]+)?$"
    )
    .expect("Regex should compile");

    static ref REAL_FILENAME_RE: Regex = Regex::new(
        r"^(?:[.](?<pid>[0-9]+))?(?:[.]t(?<tid>[0-9]+))?(?:[.]p(?<part>[0-9]+))?(?:[.](?<bbv>bb|pc))?(?:[.](?<type>out|log))(?:[.](?<base>old|base@[^.]+))?$"
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
    pub setup: Option<Assistant>,
    pub teardown: Option<Assistant>,
    pub sandbox: Option<api::Sandbox>,
    pub delay: Option<Delay>,
}

#[derive(Debug, Clone)]
pub struct ToolConfig {
    pub tool: ValgrindTool,
    pub is_enabled: bool,
    pub is_default: bool,
    pub args: ToolArgs,
    pub outfile_modifier: Option<String>,
    pub regression_config: ToolRegressionConfig,
    pub flamegraph_config: ToolFlamegraphConfig,
    pub entry_point: EntryPoint,
    pub frames: Vec<Glob>,
}

#[derive(Debug, Clone)]
pub struct ToolConfigs(pub Vec<ToolConfig>);

pub struct ToolCommand {
    tool: ValgrindTool,
    nocapture: NoCapture,
    command: Command,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToolFlamegraphConfig {
    Callgrind(FlamegraphConfig),
    None,
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

#[derive(Debug, Clone, PartialEq)]
pub enum ToolRegressionConfig {
    Callgrind(CallgrindRegressionConfig),
    Cachegrind(CachegrindRegressionConfig),
    Dhat(DhatRegressionConfig),
    None,
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
    pub fn new<T>(
        tool: ValgrindTool,
        is_enabled: bool,
        args: T,
        modifier: Option<String>,
        regression_config: ToolRegressionConfig,
        flamegraph_config: ToolFlamegraphConfig,
        entry_point: EntryPoint,
        is_default: bool,
        frames: &[String],
    ) -> Result<Self>
    where
        T: Into<ToolArgs>,
    {
        Ok(Self {
            tool,
            is_enabled,
            args: args.into(),
            outfile_modifier: modifier,
            regression_config,
            flamegraph_config,
            entry_point,
            is_default,
            frames: frames.iter().map(Into::into).collect(),
        })
    }

    #[allow(clippy::too_many_lines)]
    pub fn from_tool(
        output_format: &mut OutputFormat,
        valgrind_tool: ValgrindTool,
        tool: Option<Tool>,
        meta: &Metadata,
        base_args: &RawArgs,
        is_default: bool,
        regression_config: Option<api::ToolRegressionConfig>,
        flamegraph_config: Option<api::ToolFlamegraphConfig>,
        entry_point: Option<EntryPoint>,
    ) -> Result<Self> {
        if let Some(tool) = tool {
            if let Some(format) = tool.output_format {
                match format {
                    ToolOutputFormat::Callgrind(metrics) => {
                        output_format.callgrind =
                            metrics.into_iter().fold(IndexSet::new(), |mut acc, m| {
                                acc.extend(IndexSet::from(m));
                                acc
                            });
                    }
                    ToolOutputFormat::Cachegrind(metrics) => {
                        output_format.cachegrind =
                            metrics.into_iter().fold(IndexSet::new(), |mut acc, m| {
                                acc.extend(IndexSet::from(m));
                                acc
                            });
                    }
                    ToolOutputFormat::DHAT(metrics) => {
                        output_format.dhat = metrics.into_iter().collect();
                    }
                    ToolOutputFormat::Memcheck(metrics) => {
                        output_format.memcheck = metrics.into_iter().collect();
                    }
                    ToolOutputFormat::Helgrind(metrics) => {
                        output_format.helgrind = metrics.into_iter().collect();
                    }
                    ToolOutputFormat::DRD(metrics) => {
                        output_format.drd = metrics.into_iter().collect();
                    }
                    ToolOutputFormat::None => {}
                }
            }

            let args = match valgrind_tool {
                ValgrindTool::Callgrind => callgrind::args::Args::try_from_raw_args(&[
                    base_args,
                    &tool.raw_args,
                    &meta.args.valgrind_args.clone().unwrap_or_default(),
                    &meta.args.callgrind_args.clone().unwrap_or_default(),
                ])?
                .into(),
                ValgrindTool::Cachegrind => cachegrind::args::Args::try_from_raw_args(&[
                    base_args,
                    &tool.raw_args,
                    &meta.args.valgrind_args.clone().unwrap_or_default(),
                    &meta.args.cachegrind_args.clone().unwrap_or_default(),
                ])?
                .into(),
                ValgrindTool::DHAT => ToolArgs::try_from_raw_args(
                    ValgrindTool::DHAT,
                    &[
                        base_args,
                        &tool.raw_args,
                        &meta.args.valgrind_args.clone().unwrap_or_default(),
                        &meta.args.dhat_args.clone().unwrap_or_default(),
                    ],
                )?,
                ValgrindTool::Memcheck => ToolArgs::try_from_raw_args(
                    ValgrindTool::Memcheck,
                    &[
                        base_args,
                        &tool.raw_args,
                        &meta.args.valgrind_args.clone().unwrap_or_default(),
                        &meta.args.memcheck_args.clone().unwrap_or_default(),
                    ],
                )?,
                ValgrindTool::Helgrind => ToolArgs::try_from_raw_args(
                    ValgrindTool::Helgrind,
                    &[
                        base_args,
                        &tool.raw_args,
                        &meta.args.valgrind_args.clone().unwrap_or_default(),
                        &meta.args.helgrind_args.clone().unwrap_or_default(),
                    ],
                )?,
                ValgrindTool::DRD => ToolArgs::try_from_raw_args(
                    ValgrindTool::DRD,
                    &[
                        base_args,
                        &tool.raw_args,
                        &meta.args.valgrind_args.clone().unwrap_or_default(),
                        &meta.args.drd_args.clone().unwrap_or_default(),
                    ],
                )?,
                ValgrindTool::Massif => ToolArgs::try_from_raw_args(
                    ValgrindTool::Massif,
                    &[
                        base_args,
                        &tool.raw_args,
                        &meta.args.valgrind_args.clone().unwrap_or_default(),
                        &meta.args.massif_args.clone().unwrap_or_default(),
                    ],
                )?,
                ValgrindTool::BBV => ToolArgs::try_from_raw_args(
                    ValgrindTool::BBV,
                    &[
                        base_args,
                        &tool.raw_args,
                        &meta.args.valgrind_args.clone().unwrap_or_default(),
                        &meta.args.bbv_args.clone().unwrap_or_default(),
                    ],
                )?,
            };

            let mut regression_config = regression_config
                .or(tool.regression_config)
                .map_or(ToolRegressionConfig::None, Into::into);
            if let Some(fail_fast) = meta.args.regression_fail_fast {
                match &mut regression_config {
                    ToolRegressionConfig::Callgrind(callgrind_regression_config) => {
                        callgrind_regression_config.fail_fast = fail_fast;
                    }
                    ToolRegressionConfig::Cachegrind(cachegrind_regression_config) => {
                        cachegrind_regression_config.fail_fast = fail_fast;
                    }
                    ToolRegressionConfig::Dhat(dhat_regression_config) => {
                        dhat_regression_config.fail_fast = fail_fast;
                    }
                    ToolRegressionConfig::None => {}
                }
            }

            ToolConfig::new(
                valgrind_tool,
                is_default || tool.enable.unwrap_or(true),
                args,
                None,
                regression_config,
                flamegraph_config
                    .or(tool.flamegraph_config)
                    .map_or(ToolFlamegraphConfig::None, Into::into),
                entry_point.or(tool.entry_point).unwrap_or(EntryPoint::None),
                is_default,
                &tool.frames.unwrap_or_default(),
            )
        } else {
            let args = match valgrind_tool {
                ValgrindTool::Callgrind => callgrind::args::Args::try_from_raw_args(&[
                    base_args,
                    &meta.args.valgrind_args.clone().unwrap_or_default(),
                    &meta.args.callgrind_args.clone().unwrap_or_default(),
                ])?
                .into(),
                ValgrindTool::Cachegrind => cachegrind::args::Args::try_from_raw_args(&[
                    base_args,
                    &meta.args.valgrind_args.clone().unwrap_or_default(),
                    &meta.args.cachegrind_args.clone().unwrap_or_default(),
                ])?
                .into(),
                ValgrindTool::DHAT => ToolArgs::try_from_raw_args(
                    ValgrindTool::DHAT,
                    &[
                        base_args,
                        &meta.args.valgrind_args.clone().unwrap_or_default(),
                        &meta.args.dhat_args.clone().unwrap_or_default(),
                    ],
                )?,
                ValgrindTool::Memcheck => ToolArgs::try_from_raw_args(
                    ValgrindTool::Memcheck,
                    &[
                        base_args,
                        &meta.args.valgrind_args.clone().unwrap_or_default(),
                        &meta.args.memcheck_args.clone().unwrap_or_default(),
                    ],
                )?,
                ValgrindTool::Helgrind => ToolArgs::try_from_raw_args(
                    ValgrindTool::Helgrind,
                    &[
                        base_args,
                        &meta.args.valgrind_args.clone().unwrap_or_default(),
                        &meta.args.helgrind_args.clone().unwrap_or_default(),
                    ],
                )?,
                ValgrindTool::DRD => ToolArgs::try_from_raw_args(
                    ValgrindTool::DRD,
                    &[
                        base_args,
                        &meta.args.valgrind_args.clone().unwrap_or_default(),
                        &meta.args.drd_args.clone().unwrap_or_default(),
                    ],
                )?,
                ValgrindTool::Massif => ToolArgs::try_from_raw_args(
                    ValgrindTool::Massif,
                    &[
                        base_args,
                        &meta.args.valgrind_args.clone().unwrap_or_default(),
                        &meta.args.massif_args.clone().unwrap_or_default(),
                    ],
                )?,
                ValgrindTool::BBV => ToolArgs::try_from_raw_args(
                    ValgrindTool::BBV,
                    &[
                        base_args,
                        &meta.args.valgrind_args.clone().unwrap_or_default(),
                        &meta.args.bbv_args.clone().unwrap_or_default(),
                    ],
                )?,
            };

            let mut regression_config =
                regression_config.map_or(ToolRegressionConfig::None, Into::into);
            if let Some(fail_fast) = meta.args.regression_fail_fast {
                match &mut regression_config {
                    ToolRegressionConfig::Callgrind(callgrind_regression_config) => {
                        callgrind_regression_config.fail_fast = fail_fast;
                    }
                    ToolRegressionConfig::Cachegrind(cachegrind_regression_config) => {
                        cachegrind_regression_config.fail_fast = fail_fast;
                    }
                    ToolRegressionConfig::Dhat(dhat_regression_config) => {
                        dhat_regression_config.fail_fast = fail_fast;
                    }
                    ToolRegressionConfig::None => {}
                }
            }

            ToolConfig::new(
                valgrind_tool,
                true,
                args,
                None,
                regression_config,
                flamegraph_config.map_or(ToolFlamegraphConfig::None, Into::into),
                entry_point.unwrap_or(EntryPoint::None),
                is_default,
                &[],
            )
        }
    }

    pub fn new_default_config(
        output_format: &mut OutputFormat,
        module_path: &ModulePath,
        meta: &Metadata,
        default_tool: ValgrindTool,
        mut tool: Option<Tool>,
        default_entry_point: EntryPoint,
        valgrind_args: &RawArgs,
        default_args: &HashMap<ValgrindTool, RawArgs>,
    ) -> Result<Self> {
        match default_tool {
            ValgrindTool::Callgrind => {
                let mut base_args = default_args
                    .get(&ValgrindTool::Callgrind)
                    .cloned()
                    .unwrap_or_default();
                base_args.update(valgrind_args);

                let entry_point = tool
                    .as_ref()
                    .and_then(|t| t.entry_point.clone())
                    .unwrap_or(default_entry_point);

                match &entry_point {
                    EntryPoint::None => {}
                    EntryPoint::Default => {
                        base_args.extend_ignore_flag(&[format!("toggle-collect={DEFAULT_TOGGLE}")]);
                    }
                    EntryPoint::Custom(custom) => {
                        base_args.extend_ignore_flag(&[format!("toggle-collect={custom}")]);
                    }
                }

                ToolConfig::from_tool(
                    output_format,
                    default_tool,
                    tool,
                    meta,
                    &base_args,
                    true,
                    meta.args.callgrind_limits.clone(),
                    None,
                    Some(entry_point),
                )
            }
            ValgrindTool::Cachegrind => {
                let mut base_args = default_args
                    .get(&ValgrindTool::Cachegrind)
                    .cloned()
                    .unwrap_or_default();
                base_args.update(valgrind_args);

                ToolConfig::from_tool(
                    output_format,
                    ValgrindTool::Cachegrind,
                    tool,
                    meta,
                    &base_args,
                    true,
                    meta.args.cachegrind_limits.clone(),
                    None,
                    None, // The default entry point is currently just for callgrind
                )
            }
            ValgrindTool::DHAT => {
                let mut base_args = default_args
                    .get(&ValgrindTool::DHAT)
                    .cloned()
                    .unwrap_or_default();
                base_args.update(valgrind_args);

                let entry_point = tool
                    .as_ref()
                    .and_then(|t| t.entry_point.clone())
                    .unwrap_or(default_entry_point);

                if let EntryPoint::Default = entry_point {
                    let tool = tool.get_or_insert_with(|| Tool::new(ValgrindTool::DHAT));
                    let frames = tool.frames.get_or_insert_with(Vec::new);

                    // DHAT does not resolve function calls the same way as callgrind does. Somehow
                    // the benchmark function matched by the `DEFAULT_TOGGLE` gets sometimes inlined
                    // (although annotated with `#[inline(never)]`), so we need to fall back to the
                    // next best thing which is the function that calls the benchmark function. At
                    // this point the module path consists of `file::group::function`. The group in
                    // the path is artificial and we need the real function path within the
                    // benchmark file to create a matching glob pattern. That real path consists of
                    // `file::function::id`. The `id`-function won't be matched literally but with a
                    // wildcard to address the problem of functions with the same body being
                    // condensed into a single function by the compiler. There is no way to know
                    // which concrete `id`-function the compiler chose in the end, so we match it
                    // with a wildcard.
                    if let [first, _, last] = module_path.components()[..] {
                        frames.push(format!("{first}::{last}::*"));
                    }
                }

                ToolConfig::from_tool(
                    output_format,
                    ValgrindTool::DHAT,
                    tool,
                    meta,
                    &base_args,
                    true,
                    meta.args.dhat_limits.clone(),
                    None,
                    Some(entry_point), // The default entry point is currently just for callgrind
                )
            }
            valgrind_tool => {
                let mut base_args = default_args
                    .get(&valgrind_tool)
                    .cloned()
                    .unwrap_or_default();
                base_args.update(valgrind_args);

                ToolConfig::from_tool(
                    output_format,
                    valgrind_tool,
                    tool,
                    meta,
                    &base_args,
                    true,
                    None,
                    None,
                    None, // The default entry point is currently just for callgrind
                )
            }
        }
    }

    pub fn parse(
        &self,
        meta: &Metadata,
        output_path: &ToolOutputPath,
        parsed_old: Option<Vec<ParserOutput>>,
    ) -> Result<Profile> {
        let parser = parser_factory(self, meta.project_root.clone(), output_path);

        let parsed_new = parser.parse()?;
        let parsed_old = if let Some(parsed_old) = parsed_old {
            parsed_old
        } else {
            parser.parse_base()?
        };

        let data = match (parsed_new.is_empty(), parsed_old.is_empty()) {
            (true, false | true) => return Err(anyhow!("A new dataset should always be present")),
            (false, true) => ProfileData::new(parsed_new, None),
            (false, false) => ProfileData::new(parsed_new, Some(parsed_old)),
        };

        Ok(Profile {
            tool: self.tool,
            log_paths: output_path.to_log_output().real_paths()?,
            out_paths: output_path.real_paths()?,
            summaries: data,
            flamegraphs: vec![],
        })
    }

    fn print(
        &self,
        config: &Config,
        output_format: &OutputFormat,
        data: &ProfileData,
        baselines: &Baselines,
    ) -> Result<()> {
        VerticalFormatter::new(output_format.clone()).print(
            self.tool,
            config,
            baselines,
            data,
            self.is_default,
        )
    }
}

impl ToolConfigs {
    /// Create new `ToolConfigs`
    ///
    /// `default_entry_point` is callgrind specific and specified here because it is different for
    /// library and binary benchmarks.
    ///
    /// `default_args` should only contain command-line arguments which are different for library
    /// and binary benchmarks on a per tool basis. Usually, default arguments are part of the tool
    /// specific `Args` struct for example for callgrind [`callgrind::args::Args`] or cachegrind
    /// [`cachegrind::args::Args`].
    ///
    /// # Errors
    ///
    /// This function will return an error if the configs cannot be created
    #[expect(clippy::too_many_lines)]
    pub fn new(
        output_format: &mut OutputFormat,
        mut tools: Tools,
        module_path: &ModulePath,
        meta: &Metadata,
        default_tool: ValgrindTool,
        default_entry_point: &EntryPoint,
        valgrind_args: &RawArgs,
        default_args: &HashMap<ValgrindTool, RawArgs>,
    ) -> Result<Self> {
        let extracted_tool = tools.consume(default_tool);
        let default_tool_config = ToolConfig::new_default_config(
            output_format,
            module_path,
            meta,
            default_tool,
            extracted_tool,
            default_entry_point.clone(),
            valgrind_args,
            default_args,
        )?;

        // The tool selection from the command line or env args overwrites the tool selection from
        // the benchmark file. However, any tool configurations from the benchmark files are
        // preserved.
        let meta_tools = if meta.args.tools.is_empty() {
            tools.0
        } else {
            let mut meta_tools = Vec::with_capacity(meta.args.tools.len());
            for kind in &meta.args.tools {
                if let Some(tool) = tools.consume(*kind) {
                    meta_tools.push(tool);
                } else {
                    meta_tools.push(Tool::new(*kind));
                }
            }
            meta_tools
        };

        let mut tool_configs = ToolConfigs(vec![default_tool_config]);
        tool_configs.extend(meta_tools.into_iter().map(|mut tool| {
            let mut base_args = default_args.get(&tool.kind).cloned().unwrap_or_default();
            base_args.update(valgrind_args);

            match tool.kind {
                ValgrindTool::Callgrind => {
                    let entry_point = tool
                        .entry_point
                        .clone()
                        .unwrap_or(default_entry_point.clone());

                    match &entry_point {
                        EntryPoint::None => {}
                        EntryPoint::Default => {
                            base_args
                                .extend_ignore_flag(&[format!("toggle-collect={DEFAULT_TOGGLE}")]);
                        }
                        EntryPoint::Custom(custom) => {
                            base_args.extend_ignore_flag(&[format!("toggle-collect={custom}")]);
                        }
                    }

                    ToolConfig::from_tool(
                        output_format,
                        tool.kind,
                        Some(tool),
                        meta,
                        &base_args,
                        false,
                        meta.args.callgrind_limits.clone(),
                        None,
                        Some(entry_point),
                    )
                }
                ValgrindTool::Cachegrind => ToolConfig::from_tool(
                    output_format,
                    tool.kind,
                    Some(tool),
                    meta,
                    &base_args,
                    false,
                    meta.args.cachegrind_limits.clone(),
                    None,
                    None,
                ),
                ValgrindTool::DHAT => {
                    let entry_point = tool
                        .entry_point
                        .clone()
                        .unwrap_or(default_entry_point.clone());

                    if let EntryPoint::Default = entry_point {
                        let frames = tool.frames.get_or_insert_with(Vec::new);

                        // For details see comment in `ToolConfig::new_default_config`
                        if let [first, _, last] = module_path.components()[..] {
                            frames.push(format!("{first}::{last}::*"));
                        }
                    }

                    ToolConfig::from_tool(
                        output_format,
                        tool.kind,
                        Some(tool),
                        meta,
                        &base_args,
                        false,
                        meta.args.dhat_limits.clone(),
                        None,
                        Some(entry_point),
                    )
                }
                _ => ToolConfig::from_tool(
                    output_format,
                    tool.kind,
                    Some(tool),
                    meta,
                    &base_args,
                    false,
                    None,
                    None,
                    None,
                ),
            }
        }))?;

        Ok(tool_configs)
    }

    pub fn has_tools_enabled(&self) -> bool {
        self.0.iter().any(|t| t.is_enabled)
    }

    /// Return true if there are multiple tools configured and are enabled
    pub fn has_multiple(&self) -> bool {
        self.0.len() > 1 && self.0.iter().filter(|f| f.is_enabled).count() > 1
    }

    pub fn output_paths(&self, output_path: &ToolOutputPath) -> Vec<ToolOutputPath> {
        self.0
            .iter()
            .filter(|t| t.is_enabled)
            .map(|t| output_path.to_tool_output(t.tool))
            .collect()
    }

    pub fn extend(&mut self, iter: impl Iterator<Item = Result<ToolConfig>>) -> Result<()> {
        for a in iter {
            self.0.push(a?);
        }

        Ok(())
    }

    fn print_headline(&self, tool_config: &ToolConfig, output_format: &OutputFormat) {
        if output_format.is_default()
            && (self.has_multiple() || tool_config.tool != ValgrindTool::Callgrind)
        {
            let mut formatter = VerticalFormatter::new(output_format.clone());
            formatter.format_tool_headline(tool_config.tool);
            formatter.print_buffer();
        }
    }

    /// Check for regressions as defined in [`RegressionConfig`] and print an error if a regression
    /// occurred
    ///
    /// # Panics
    ///
    /// Checking performance regressions for other tools than callgrind and cachegrind is not
    /// implemented and panics
    fn check_and_print_regressions(
        tool_regression_config: &ToolRegressionConfig,
        tool_total: &ProfileTotal,
    ) -> Vec<ToolRegression> {
        match (tool_regression_config, &tool_total.summary) {
            (
                ToolRegressionConfig::Callgrind(callgrind_regression_config),
                ToolMetricSummary::Callgrind(metrics_summary),
            ) => callgrind_regression_config.check_and_print(metrics_summary),
            (
                ToolRegressionConfig::Cachegrind(cachegrind_regression_config),
                ToolMetricSummary::Cachegrind(metrics_summary),
            ) => cachegrind_regression_config.check_and_print(metrics_summary),
            (
                ToolRegressionConfig::Dhat(dhat_regression_config),
                ToolMetricSummary::Dhat(metrics_summary),
            ) => dhat_regression_config.check_and_print(metrics_summary),
            (ToolRegressionConfig::None, _) => vec![],
            _ => {
                panic!("The summary type should match the regression config")
            }
        }
    }

    pub fn run_loaded_vs_base(
        &self,
        title: &str,
        baseline: &BaselineName,
        loaded_baseline: &BaselineName,
        mut benchmark_summary: BenchmarkSummary,
        baselines: &Baselines,
        config: &Config,
        output_path: &ToolOutputPath,
        output_format: &OutputFormat,
    ) -> Result<BenchmarkSummary> {
        for tool_config in self.0.iter().filter(|t| t.is_enabled) {
            self.print_headline(tool_config, output_format);

            let tool = tool_config.tool;
            let output_path = output_path.to_tool_output(tool);

            let mut profile = tool_config.parse(&config.meta, &output_path, None)?;

            tool_config.print(config, output_format, &profile.summaries, baselines)?;
            profile.summaries.total.regressions = Self::check_and_print_regressions(
                &tool_config.regression_config,
                &profile.summaries.total,
            );

            if ValgrindTool::Callgrind == tool {
                if let ToolFlamegraphConfig::Callgrind(flamegraph_config) =
                    &tool_config.flamegraph_config
                {
                    profile.flamegraphs = LoadBaselineFlamegraphGenerator {
                        loaded_baseline: loaded_baseline.clone(),
                        baseline: baseline.clone(),
                    }
                    .create(
                        &Flamegraph::new(title.to_owned(), flamegraph_config.to_owned()),
                        &output_path,
                        (tool_config.entry_point == EntryPoint::Default)
                            .then(Sentinel::default)
                            .as_ref(),
                        &config.meta.project_root,
                    )?;
                }
            }

            benchmark_summary.profiles.push(profile);

            let log_path = output_path.to_log_output();
            log_path.dump_log(log::Level::Info, &mut stderr())?;
        }

        Ok(benchmark_summary)
    }

    pub fn run(
        &self,
        title: &str,
        mut benchmark_summary: BenchmarkSummary,
        baselines: &Baselines,
        baseline_kind: &BaselineKind,
        config: &Config,
        executable: &Path,
        executable_args: &[OsString],
        run_options: &RunOptions,
        output_path: &ToolOutputPath,
        save_baseline: bool,
        module_path: &ModulePath,
        output_format: &OutputFormat,
    ) -> Result<BenchmarkSummary> {
        for tool_config in self.0.iter().filter(|t| t.is_enabled) {
            // Print the headline as soon as possible, so if there are any errors, the errors shown
            // in the terminal output can be associated with the tool
            self.print_headline(tool_config, output_format);

            let tool = tool_config.tool;

            let nocapture = if tool_config.is_default {
                config.meta.args.nocapture
            } else {
                NoCapture::False
            };
            let command = ToolCommand::new(tool, &config.meta, nocapture);

            let output_path = output_path.to_tool_output(tool);

            let parser =
                parser_factory(tool_config, config.meta.project_root.clone(), &output_path);
            let parsed_old = parser.parse_base()?;

            let log_path = output_path.to_log_output();

            if save_baseline {
                output_path.clear()?;
                log_path.clear()?;
            }

            // We're implicitly applying the default here: In the absence of a user provided sandbox
            // we don't run the benchmarks in a sandbox. Everything from here on runs
            // with the current directory set to the sandbox directory until the sandbox
            // is reset.
            let sandbox = run_options
                .sandbox
                .as_ref()
                .map(|sandbox| Sandbox::setup(sandbox, &config.meta))
                .transpose()?;

            let mut child = run_options
                .setup
                .as_ref()
                .map_or(Ok(None), |setup| setup.run(config, module_path))?;

            if let Some(delay) = run_options.delay.as_ref() {
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

            if let Some(teardown) = run_options.teardown.as_ref() {
                teardown.run(config, module_path)?;
            }

            // We print the no capture footer after the teardown to keep the output consistent with
            // library benchmarks.
            print_no_capture_footer(
                nocapture,
                run_options.stdout.as_ref(),
                run_options.stderr.as_ref(),
            );

            if let Some(sandbox) = sandbox {
                sandbox.reset()?;
            }

            let mut profile = tool_config.parse(&config.meta, &output_path, Some(parsed_old))?;

            tool_config.print(config, output_format, &profile.summaries, baselines)?;
            profile.summaries.total.regressions = Self::check_and_print_regressions(
                &tool_config.regression_config,
                &profile.summaries.total,
            );

            if tool_config.tool == ValgrindTool::Callgrind {
                if save_baseline {
                    let BaselineKind::Name(baseline) = baseline_kind.clone() else {
                        panic!("A baseline with name should be present");
                    };
                    if let ToolFlamegraphConfig::Callgrind(flamegraph_config) =
                        &tool_config.flamegraph_config
                    {
                        profile.flamegraphs = SaveBaselineFlamegraphGenerator { baseline }.create(
                            &Flamegraph::new(title.to_owned(), flamegraph_config.to_owned()),
                            &output_path,
                            (tool_config.entry_point == EntryPoint::Default)
                                .then(Sentinel::default)
                                .as_ref(),
                            &config.meta.project_root,
                        )?;
                    }
                } else if let ToolFlamegraphConfig::Callgrind(flamegraph_config) =
                    &tool_config.flamegraph_config
                {
                    profile.flamegraphs = BaselineFlamegraphGenerator {
                        baseline_kind: baseline_kind.clone(),
                    }
                    .create(
                        &Flamegraph::new(title.to_owned(), flamegraph_config.to_owned()),
                        &output_path,
                        (tool_config.entry_point == EntryPoint::Default)
                            .then(Sentinel::default)
                            .as_ref(),
                        &config.meta.project_root,
                    )?;
                } else {
                    // do nothing
                }
            }

            benchmark_summary.profiles.push(profile);

            output.dump_log(log::Level::Info);
            log_path.dump_log(log::Level::Info, &mut stderr())?;
        }

        Ok(benchmark_summary)
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
    /// The `name` is supposed to be the name of the benchmark function. If a benchmark id is
    /// present join both with a dot as separator to get the final `name`.
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
        self.real_paths().is_ok_and(|p| !p.is_empty())
    }

    pub fn is_multiple(&self) -> bool {
        self.real_paths().is_ok_and(|p| p.len() > 1)
    }

    #[must_use]
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

    /// Convert this tool output to the output of another tool
    ///
    /// A tool with no `*.out` file is log-file based. If the other tool is a out-file based tool
    /// the [`ToolOutputPathKind`] will be converted and vice-versa. The "old" (base) type (a tool
    /// output converted with [`ToolOutputPath::to_base_path`]) will be converted to a new
    /// `ToolOutputPath`.
    #[must_use]
    pub fn to_tool_output(&self, tool: ValgrindTool) -> Self {
        let kind = if tool.has_output_file() {
            match &self.kind {
                ToolOutputPathKind::Log | ToolOutputPathKind::OldLog => ToolOutputPathKind::Out,
                ToolOutputPathKind::BaseLog(name) => ToolOutputPathKind::Base(name.clone()),
                kind => kind.clone(),
            }
        } else {
            match &self.kind {
                ToolOutputPathKind::Out | ToolOutputPathKind::OldOut => ToolOutputPathKind::Log,
                ToolOutputPathKind::Base(name) => ToolOutputPathKind::BaseLog(name.clone()),
                kind => kind.clone(),
            }
        };
        Self {
            tool,
            kind,
            baseline_kind: self.baseline_kind.clone(),
            name: self.name.clone(),
            dir: self.dir.clone(),
            modifiers: self.modifiers.clone(),
        }
    }

    /// Convert this tool output to the according log output
    ///
    /// All tools have a log output even the ones which are out-file based.
    #[must_use]
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

    /// Return the path to the log file for the given `path`
    ///
    /// `path` is supposed to be a path to a valid file in the directory of this [`ToolOutputPath`].
    pub fn log_path_of(&self, path: &Path) -> Option<PathBuf> {
        let file_name = path.strip_prefix(&self.dir).ok()?;
        if let Some(suffix) = self.strip_prefix(&file_name.to_string_lossy()) {
            let caps = REAL_FILENAME_RE.captures(suffix)?;
            if let Some(kind) = caps.name("type") {
                match kind.as_str() {
                    "out" => {
                        let mut string = self.prefix();
                        for s in [
                            caps.name("pid").map(|c| format!(".{}", c.as_str())),
                            Some(".log".to_owned()),
                            caps.name("base").map(|c| format!(".{}", c.as_str())),
                        ]
                        .iter()
                        .filter_map(|s| s.as_ref())
                        {
                            string.push_str(s);
                        }

                        return Some(self.dir.join(string));
                    }
                    _ => return Some(path.to_owned()),
                }
            }
        }

        None
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

    /// This method can only be used to create the path passed to the tools
    ///
    /// The modifiers are extrapolated by the tools and won't match any real path name.
    pub fn extension(&self) -> String {
        match (&self.kind, self.modifiers.is_empty()) {
            (ToolOutputPathKind::Out, true) => "out".to_owned(),
            (ToolOutputPathKind::Out, false) => format!("out.{}", self.modifiers.join(".")),
            (ToolOutputPathKind::Log, true) => "log".to_owned(),
            (ToolOutputPathKind::Log, false) => format!("log.{}", self.modifiers.join(".")),
            (ToolOutputPathKind::OldOut, true) => "out.old".to_owned(),
            (ToolOutputPathKind::OldOut, false) => format!("out.old.{}", self.modifiers.join(".")),
            (ToolOutputPathKind::OldLog, true) => "log.old".to_owned(),
            (ToolOutputPathKind::OldLog, false) => format!("log.old.{}", self.modifiers.join(".")),
            (ToolOutputPathKind::BaseLog(name), true) => {
                format!("log.base@{name}")
            }
            (ToolOutputPathKind::BaseLog(name), false) => {
                format!("log.base@{name}.{}", self.modifiers.join("."))
            }
            (ToolOutputPathKind::Base(name), true) => format!("out.base@{name}"),
            (ToolOutputPathKind::Base(name), false) => {
                format!("out.base@{name}.{}", self.modifiers.join("."))
            }
        }
    }

    #[must_use]
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

    // Return the unexpanded path usable as input for `--callgrind-out-file`, ...
    //
    // The path returned by this method does not necessarily have to exist and can include modifiers
    // like `%p`. Use [`Self::real_paths`] to get the real and existing (possibly multiple) paths to
    // the output files of the respective tool.
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

    /// Return the file name prefix as in `<tool>.<name>`
    pub fn prefix(&self) -> String {
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

            // Silently ignore all paths which don't follow this scheme, for example
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

    pub fn real_paths_with_modifier(&self) -> Result<Vec<(PathBuf, Option<String>)>> {
        let mut paths = vec![];
        for entry in self.walk_dir()? {
            let file_name = entry.file_name().to_string_lossy().to_string();

            // Silently ignore all paths which don't follow this scheme, for example
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

                paths.push((
                    entry.path(),
                    modifiers.and_then(|s| (!s.is_empty()).then(|| s.to_owned())),
                ));
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
    /// fast. Additionally, `callgrind` might change the naming scheme of its files, so using the
    /// headers makes us more independent of a specific valgrind/callgrind version.
    #[allow(clippy::too_many_lines)]
    pub fn sanitize_callgrind(&self) -> Result<()> {
        // path, part
        type Grouped = (PathBuf, Option<u64>);
        // base (i.e. base@default) => pid => thread => vec: path, part
        type Group =
            HashMap<Option<String>, HashMap<Option<i32>, HashMap<Option<usize>, Vec<Grouped>>>>;

        // To figure out if there are multiple pids/parts/threads present, it's necessary to group
        // the files in this map. The order doesn't matter since we only rename the original file
        // names, which doesn't need to follow a specific order.
        //
        // At first, we group by (out|log), then base, then pid and then by part in different
        // hashmaps. The threads are grouped in a vector.
        let mut groups: HashMap<String, Group> = HashMap::new();

        for entry in self.walk_dir()? {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            let Some(haystack) = self.strip_prefix(&file_name) else {
                continue;
            };

            if let Some(caps) = CALLGRIND_ORIG_FILENAME_RE.captures(haystack) {
                // Callgrind sometimes creates empty files for no reason. We clean them
                // up here
                if entry.metadata()?.size() == 0 {
                    std::fs::remove_file(entry.path())?;
                    continue;
                }

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

                let out_type = caps
                    .name("type")
                    .expect("A out|log type should be present")
                    .as_str();

                if out_type == ".out" {
                    let properties = parse_header(
                        &mut BufReader::new(File::open(entry.path())?)
                            .lines()
                            .map(Result::unwrap),
                    )?;
                    if let Some(bases) = groups.get_mut(out_type) {
                        if let Some(pids) = bases.get_mut(&base) {
                            if let Some(threads) = pids.get_mut(&properties.pid) {
                                if let Some(parts) = threads.get_mut(&properties.thread) {
                                    parts.push((entry.path(), properties.part));
                                } else {
                                    threads.insert(
                                        properties.thread,
                                        vec![(entry.path(), properties.part)],
                                    );
                                }
                            } else {
                                pids.insert(
                                    properties.pid,
                                    HashMap::from([(
                                        properties.thread,
                                        vec![(entry.path(), properties.part)],
                                    )]),
                                );
                            }
                        } else {
                            bases.insert(
                                base.clone(),
                                HashMap::from([(
                                    properties.pid,
                                    HashMap::from([(
                                        properties.thread,
                                        vec![(entry.path(), properties.part)],
                                    )]),
                                )]),
                            );
                        }
                    } else {
                        groups.insert(
                            out_type.to_owned(),
                            HashMap::from([(
                                base.clone(),
                                HashMap::from([(
                                    properties.pid,
                                    HashMap::from([(
                                        properties.thread,
                                        vec![(entry.path(), properties.part)],
                                    )]),
                                )]),
                            )]),
                        );
                    }
                } else {
                    let pid = caps.name("pid").map(|m| {
                        m.as_str()[2..]
                            .parse::<i32>()
                            .expect("The pid from the match should be number")
                    });

                    // The log files don't expose any information about parts or threads, so
                    // these are grouped under the `None` key
                    if let Some(bases) = groups.get_mut(out_type) {
                        if let Some(pids) = bases.get_mut(&base) {
                            if let Some(threads) = pids.get_mut(&pid) {
                                if let Some(parts) = threads.get_mut(&None) {
                                    parts.push((entry.path(), None));
                                } else {
                                    threads.insert(None, vec![(entry.path(), None)]);
                                }
                            } else {
                                pids.insert(
                                    pid,
                                    HashMap::from([(None, vec![(entry.path(), None)])]),
                                );
                            }
                        } else {
                            bases.insert(
                                base.clone(),
                                HashMap::from([(
                                    pid,
                                    HashMap::from([(None, vec![(entry.path(), None)])]),
                                )]),
                            );
                        }
                    } else {
                        groups.insert(
                            out_type.to_owned(),
                            HashMap::from([(
                                base.clone(),
                                HashMap::from([(
                                    pid,
                                    HashMap::from([(None, vec![(entry.path(), None)])]),
                                )]),
                            )]),
                        );
                    }
                }
            }
        }

        for (out_type, types) in groups {
            for (base, bases) in types {
                let multiple_pids = bases.len() > 1;

                for (pid, threads) in bases {
                    let multiple_threads = threads.len() > 1;

                    for (thread, parts) in &threads {
                        let multiple_parts = parts.len() > 1;

                        for (orig_path, part) in parts {
                            let mut new_file_name = self.prefix();

                            if multiple_pids {
                                if let Some(pid) = pid {
                                    write!(new_file_name, ".{pid}").unwrap();
                                }
                            }

                            if multiple_threads {
                                if let Some(thread) = thread {
                                    let width = threads.len().ilog10() as usize + 1;
                                    write!(new_file_name, ".t{thread:0width$}").unwrap();
                                }

                                if !multiple_parts {
                                    if let Some(part) = part {
                                        let width = parts.len().ilog10() as usize + 1;
                                        write!(new_file_name, ".p{part:0width$}").unwrap();
                                    }
                                }
                            }

                            if multiple_parts {
                                if !multiple_threads {
                                    if let Some(thread) = thread {
                                        let width = threads.len().ilog10() as usize + 1;
                                        write!(new_file_name, ".t{thread:0width$}").unwrap();
                                    }
                                }

                                if let Some(part) = part {
                                    let width = parts.len().ilog10() as usize + 1;
                                    write!(new_file_name, ".p{part:0width$}").unwrap();
                                }
                            }

                            new_file_name.push_str(&out_type);
                            if let Some(base) = &base {
                                new_file_name.push_str(base);
                            }

                            let from = orig_path;
                            let to = from.with_file_name(new_file_name);

                            std::fs::rename(from, to)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // Sanitize bbv file names
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
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    #[allow(clippy::too_many_lines)]
    pub fn sanitize_bbv(&self) -> Result<()> {
        // path, thread,
        type Grouped = (PathBuf, String);
        // key: bbv_type => key: pid
        type Group =
            HashMap<Option<String>, HashMap<Option<String>, HashMap<Option<String>, Vec<Grouped>>>>;

        // key: .(out|log)
        let mut groups: HashMap<String, Group> = HashMap::new();
        for entry in self.walk_dir()? {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            let Some(haystack) = self.strip_prefix(&file_name) else {
                continue;
            };

            if let Some(caps) = BBV_ORIG_FILENAME_RE.captures(haystack) {
                if entry.metadata()?.size() == 0 {
                    std::fs::remove_file(entry.path())?;
                    continue;
                }

                // Don't sanitize old files.
                let base = if let Some(base) = caps.name("base") {
                    if base.as_str() == ".old" {
                        continue;
                    }

                    Some(base.as_str().to_owned())
                } else {
                    None
                };

                let out_type = caps.name("type").unwrap().as_str();
                let bbv_type = caps.name("bbv_type").map(|m| m.as_str().to_owned());
                let pid = caps.name("pid").map(|p| format!(".{}", &p.as_str()[2..]));

                let thread = caps
                    .name("thread")
                    .map_or_else(|| ".1".to_owned(), |t| t.as_str().to_owned());

                if let Some(bases) = groups.get_mut(out_type) {
                    if let Some(bbv_types) = bases.get_mut(&base) {
                        if let Some(pids) = bbv_types.get_mut(&bbv_type) {
                            if let Some(threads) = pids.get_mut(&pid) {
                                threads.push((entry.path(), thread));
                            } else {
                                pids.insert(pid, vec![(entry.path(), thread)]);
                            }
                        } else {
                            bbv_types.insert(
                                bbv_type.clone(),
                                HashMap::from([(pid, vec![(entry.path(), thread)])]),
                            );
                        }
                    } else {
                        bases.insert(
                            base.clone(),
                            HashMap::from([(
                                bbv_type.clone(),
                                HashMap::from([(pid, vec![(entry.path(), thread)])]),
                            )]),
                        );
                    }
                } else {
                    groups.insert(
                        out_type.to_owned(),
                        HashMap::from([(
                            base.clone(),
                            HashMap::from([(
                                bbv_type.clone(),
                                HashMap::from([(pid, vec![(entry.path(), thread)])]),
                            )]),
                        )]),
                    );
                }
            }
        }

        for (out_type, bases) in groups {
            for (base, bbv_types) in bases {
                for (bbv_type, pids) in &bbv_types {
                    let multiple_pids = pids.len() > 1;

                    for (pid, threads) in pids {
                        let multiple_threads = threads.len() > 1;

                        for (orig_path, thread) in threads {
                            let mut new_file_name = self.prefix();

                            if multiple_pids {
                                if let Some(pid) = pid.as_ref() {
                                    write!(new_file_name, "{pid}").unwrap();
                                }
                            }

                            if multiple_threads
                                && bbv_type.as_ref().is_some_and(|b| b.starts_with(".bb"))
                            {
                                let width = threads.len().ilog10() as usize + 1;

                                let thread = thread[1..]
                                    .parse::<usize>()
                                    .expect("The thread from the regex should be a number");

                                write!(new_file_name, ".t{thread:0width$}").unwrap();
                            }

                            if let Some(bbv_type) = &bbv_type {
                                new_file_name.push_str(bbv_type);
                            }

                            new_file_name.push_str(&out_type);

                            if let Some(base) = &base {
                                new_file_name.push_str(base);
                            }

                            let from = orig_path;
                            let to = from.with_file_name(new_file_name);

                            std::fs::rename(from, to)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Sanitize file names of all tools if not sanitized by a more specific method
    ///
    /// The pids are removed from the file name if there was only a single process (pid).
    /// Additionally, we check for empty files and remove them.
    pub fn sanitize_generic(&self) -> Result<()> {
        // key: base => vec: path, pid
        type Group = HashMap<Option<String>, Vec<(PathBuf, Option<String>)>>;

        // key: .(out|log)
        let mut groups: HashMap<String, Group> = HashMap::new();
        for entry in self.walk_dir()? {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            let Some(haystack) = self.strip_prefix(&file_name) else {
                continue;
            };

            if let Some(caps) = GENERIC_ORIG_FILENAME_RE.captures(haystack) {
                if entry.metadata()?.size() == 0 {
                    std::fs::remove_file(entry.path())?;
                    continue;
                }

                // Don't sanitize old files.
                let base = if let Some(base) = caps.name("base") {
                    if base.as_str() == ".old" {
                        continue;
                    }

                    Some(base.as_str().to_owned())
                } else {
                    None
                };

                let out_type = caps.name("type").unwrap().as_str();
                let pid = caps.name("pid").map(|p| format!(".{}", &p.as_str()[2..]));

                if let Some(bases) = groups.get_mut(out_type) {
                    if let Some(pids) = bases.get_mut(&base) {
                        pids.push((entry.path(), pid));
                    } else {
                        bases.insert(base, vec![(entry.path(), pid)]);
                    }
                } else {
                    groups.insert(
                        out_type.to_owned(),
                        HashMap::from([(base, vec![(entry.path(), pid)])]),
                    );
                }
            }
        }

        for (out_type, bases) in groups {
            for (base, pids) in bases {
                let multiple_pids = pids.len() > 1;
                for (orig_path, pid) in pids {
                    let mut new_file_name = self.prefix();

                    if multiple_pids {
                        if let Some(pid) = pid.as_ref() {
                            write!(new_file_name, "{pid}").unwrap();
                        }
                    }

                    new_file_name.push_str(&out_type);

                    if let Some(base) = &base {
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

    /// Sanitize file names for a specific tool
    ///
    /// Empty files are cleaned up. For more details on a specific tool see the respective
    /// sanitize_<tool> method.
    pub fn sanitize(&self) -> Result<()> {
        match self.tool {
            ValgrindTool::Callgrind => self.sanitize_callgrind()?,
            ValgrindTool::BBV => self.sanitize_bbv()?,
            _ => self.sanitize_generic()?,
        }

        Ok(())
    }
}

impl Display for ToolOutputPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.to_path().display()))
    }
}

impl ToolRegressionConfig {
    pub fn is_fail_fast(&self) -> bool {
        match self {
            ToolRegressionConfig::Callgrind(regression_config) => regression_config.fail_fast,
            ToolRegressionConfig::Cachegrind(regression_config) => regression_config.fail_fast,
            ToolRegressionConfig::Dhat(regression_config) => regression_config.fail_fast,
            ToolRegressionConfig::None => false,
        }
    }
}

impl From<api::ToolRegressionConfig> for ToolRegressionConfig {
    fn from(value: api::ToolRegressionConfig) -> Self {
        match value {
            api::ToolRegressionConfig::Callgrind(regression_config) => {
                Self::Callgrind(regression_config.into())
            }
            api::ToolRegressionConfig::Cachegrind(regression_config) => {
                Self::Cachegrind(regression_config.into())
            }
            api::ToolRegressionConfig::Dhat(regression_config) => {
                Self::Dhat(regression_config.into())
            }
            api::ToolRegressionConfig::None => Self::None,
        }
    }
}

impl From<Option<FlamegraphConfig>> for ToolFlamegraphConfig {
    fn from(value: Option<FlamegraphConfig>) -> Self {
        match value {
            Some(config) => ToolFlamegraphConfig::Callgrind(config),
            None => ToolFlamegraphConfig::None,
        }
    }
}

impl From<api::ToolFlamegraphConfig> for ToolFlamegraphConfig {
    fn from(value: api::ToolFlamegraphConfig) -> Self {
        match value {
            api::ToolFlamegraphConfig::Callgrind(flamegraph_config) => {
                Self::Callgrind(flamegraph_config.into())
            }
            api::ToolFlamegraphConfig::None => Self::None,
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

pub fn parser_factory(
    tool_config: &ToolConfig,
    root_dir: PathBuf,
    output_path: &ToolOutputPath,
) -> Box<dyn Parser> {
    match tool_config.tool {
        ValgrindTool::Callgrind => Box::new(callgrind::summary_parser::SummaryParser {
            output_path: output_path.clone(),
        }),
        ValgrindTool::Cachegrind => Box::new(cachegrind::summary_parser::SummaryParser {
            output_path: output_path.clone(),
        }),
        ValgrindTool::DHAT => {
            if tool_config.entry_point == EntryPoint::None && tool_config.frames.is_empty() {
                Box::new(DhatLogfileParser::new(
                    output_path.to_log_output(),
                    root_dir,
                ))
            } else {
                Box::new(JsonParser::new(
                    output_path.clone(),
                    tool_config.entry_point.clone(),
                    tool_config.frames.clone(),
                ))
            }
        }
        ValgrindTool::Memcheck | ValgrindTool::DRD | ValgrindTool::Helgrind => {
            Box::new(ErrorMetricLogfileParser {
                output_path: output_path.to_log_output(),
                root_dir,
            })
        }
        _ => Box::new(GenericLogfileParser {
            output_path: output_path.to_log_output(),
            root_dir,
        }),
    }
}

#[cfg(test)]
mod tests {

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::out(".out")]
    #[case::out_with_pid(".out.#1234")]
    #[case::out_with_part(".out.1")]
    #[case::out_with_thread(".out-01")]
    #[case::out_with_part_and_thread(".out.1-01")]
    #[case::out_base(".out.base@default")]
    #[case::out_base_with_pid(".out.base@default.#1234")]
    #[case::out_base_with_part(".out.base@default.1")]
    #[case::out_base_with_thread(".out.base@default-01")]
    #[case::out_base_with_part_and_thread(".out.base@default.1-01")]
    #[case::log(".log")]
    #[case::log_with_pid(".log.#1234")]
    #[case::log_base(".log.base@default")]
    #[case::log_base_with_pid(".log.base@default.#1234")]
    fn test_callgrind_filename_regex(#[case] haystack: &str) {
        assert!(CALLGRIND_ORIG_FILENAME_RE.is_match(haystack));
    }

    #[rstest]
    #[case::bb_out(".out.bb")]
    #[case::bb_out_with_pid(".out.bb.#1234")]
    #[case::bb_out_with_pid_and_thread(".out.bb.#1234.1")]
    #[case::bb_out_with_thread(".out.bb.1")]
    #[case::pc_out(".out.pc")]
    #[case::log(".log")]
    #[case::log_with_pid(".log.#1234")]
    fn test_bbv_filename_regex(#[case] haystack: &str) {
        assert!(BBV_ORIG_FILENAME_RE.is_match(haystack));
    }

    #[rstest]
    #[case::out(".out", vec![("type", "out")])]
    #[case::pid_out(".2049595.out", vec![("pid", "2049595"), ("type", "out")])]
    #[case::pid_thread_out(".2049595.t1.out", vec![("pid", "2049595"), ("tid", "1"), ("type", "out")])]
    #[case::pid_thread_part_out(".2049595.t1.p1.out", vec![("pid", "2049595"), ("tid", "1"), ("part", "1"), ("type", "out")])]
    #[case::out_old(".out.old", vec![("type", "out"), ("base", "old")])]
    #[case::pid_out_old(".2049595.out.old", vec![("pid", "2049595"), ("type", "out"), ("base", "old")])]
    #[case::pid_thread_out_old(".2049595.t1.out.old", vec![("pid", "2049595"), ("tid", "1"), ("type", "out"), ("base", "old")])]
    #[case::pid_thread_part_out_old(".2049595.t1.p1.out.old", vec![("pid", "2049595"), ("tid", "1"), ("part", "1"), ("type", "out"), ("base", "old")])]
    #[case::out_base(".out.base@name", vec![("type", "out"), ("base", "base@name")])]
    #[case::pid_out_base(".2049595.out.base@name", vec![("pid", "2049595"), ("type", "out"), ("base", "base@name")])]
    #[case::pid_thread_out_base(".2049595.t1.out.base@name", vec![("pid", "2049595"), ("tid", "1"), ("type", "out"), ("base", "base@name")])]
    #[case::pid_thread_part_out_base(".2049595.t1.p1.out.base@name", vec![("pid", "2049595"), ("tid", "1"), ("part", "1"), ("type", "out"), ("base", "base@name")])]
    #[case::bb_out(".bb.out", vec![("bbv", "bb"), ("type", "out")])]
    #[case::pc_out(".pc.out", vec![("bbv", "pc"), ("type", "out")])]
    #[case::pid_bb_out(".123.bb.out", vec![("pid", "123"), ("bbv", "bb"), ("type", "out")])]
    #[case::pid_thread_bb_out(".123.t1.bb.out", vec![("pid", "123"), ("tid", "1"), ("bbv", "bb"), ("type", "out")])]
    #[case::log(".log", vec![("type", "log")])]
    fn test_real_file_name_regex(#[case] haystack: &str, #[case] expected: Vec<(&str, &str)>) {
        assert!(REAL_FILENAME_RE.is_match(haystack));

        let caps = REAL_FILENAME_RE.captures(haystack).unwrap();
        for (name, value) in expected {
            assert_eq!(caps.name(name).unwrap().as_str(), value);
        }
    }

    #[rstest]
    #[case::out(
        ValgrindTool::Callgrind,
        "callgrind.bench_thread_in_subprocess.two.out",
        "callgrind.bench_thread_in_subprocess.two.log"
    )]
    #[case::out_old(
        ValgrindTool::Callgrind,
        "callgrind.bench_thread_in_subprocess.two.out.old",
        "callgrind.bench_thread_in_subprocess.two.log.old"
    )]
    #[case::pid_out(
        ValgrindTool::Callgrind,
        "callgrind.bench_thread_in_subprocess.two.123.out",
        "callgrind.bench_thread_in_subprocess.two.123.log"
    )]
    #[case::pid_tid_out(
        ValgrindTool::Callgrind,
        "callgrind.bench_thread_in_subprocess.two.123.t1.out",
        "callgrind.bench_thread_in_subprocess.two.123.log"
    )]
    #[case::pid_tid_part_out(
        ValgrindTool::Callgrind,
        "callgrind.bench_thread_in_subprocess.two.123.t1.p2.out",
        "callgrind.bench_thread_in_subprocess.two.123.log"
    )]
    #[case::pid_out_old(
        ValgrindTool::Callgrind,
        "callgrind.bench_thread_in_subprocess.two.123.out.old",
        "callgrind.bench_thread_in_subprocess.two.123.log.old"
    )]
    #[case::pid_tid_part_out_old(
        ValgrindTool::Callgrind,
        "callgrind.bench_thread_in_subprocess.two.123.t1.p2.out.old",
        "callgrind.bench_thread_in_subprocess.two.123.log.old"
    )]
    #[case::bb_out(
        ValgrindTool::BBV,
        "exp-bbv.bench_thread_in_subprocess.two.bb.out",
        "exp-bbv.bench_thread_in_subprocess.two.log"
    )]
    #[case::bb_pid_out(
        ValgrindTool::BBV,
        "exp-bbv.bench_thread_in_subprocess.two.123.bb.out",
        "exp-bbv.bench_thread_in_subprocess.two.123.log"
    )]
    #[case::bb_pid_tid_out(
        ValgrindTool::BBV,
        "exp-bbv.bench_thread_in_subprocess.two.123.t1.bb.out",
        "exp-bbv.bench_thread_in_subprocess.two.123.log"
    )]
    fn test_tool_output_path_log_path_of(
        #[case] tool: ValgrindTool,
        #[case] input: PathBuf,
        #[case] expected: PathBuf,
    ) {
        let output_path = ToolOutputPath::new(
            ToolOutputPathKind::Out,
            tool,
            &BaselineKind::Old,
            &PathBuf::from("/root"),
            &ModulePath::new("hello::world"),
            "bench_thread_in_subprocess.two",
        );
        let expected = output_path.dir.join(expected);
        let actual = output_path
            .log_path_of(&output_path.dir.join(input))
            .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_tool_output_path_log_path_of_when_not_in_dir_then_none() {
        let output_path = ToolOutputPath::new(
            ToolOutputPathKind::Out,
            ValgrindTool::Callgrind,
            &BaselineKind::Old,
            &PathBuf::from("/root"),
            &ModulePath::new("hello::world"),
            "bench_thread_in_subprocess.two",
        );

        assert!(output_path
            .log_path_of(&PathBuf::from(
                "/root/not/here/bench_thread_in_subprocess.two/callgrind.\
                 bench_thread_in_subprocess.two.out"
            ))
            .is_none());
    }

    #[test]
    fn test_tool_output_path_log_path_of_when_log_then_same() {
        let output_path = ToolOutputPath::new(
            ToolOutputPathKind::Log,
            ValgrindTool::Callgrind,
            &BaselineKind::Old,
            &PathBuf::from("/root"),
            &ModulePath::new("hello::world"),
            "bench_thread_in_subprocess.two",
        );
        let path = PathBuf::from(
            "/root/hello/world/bench_thread_in_subprocess.two/callgrind.\
             bench_thread_in_subprocess.two.log",
        );

        assert_eq!(output_path.log_path_of(&path), Some(path));
    }
}
