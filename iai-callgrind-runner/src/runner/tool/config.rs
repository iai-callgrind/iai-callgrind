//! The module containing the [`ToolConfig`] and other related elements

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::stderr;
use std::path::Path;

use anyhow::{anyhow, Result};
use indexmap::IndexSet;

use super::args::ToolArgs;
use super::parser::{parser_factory, ParserOutput};
use super::path::ToolOutputPath;
use super::regression::{RegressionConfig, ToolRegressionConfig};
use super::run::{RunOptions, ToolCommand};
use crate::api::{self, EntryPoint, RawArgs, Tool, ToolOutputFormat, Tools, ValgrindTool};
use crate::runner::args::NoCapture;
use crate::runner::callgrind::flamegraph::{
    BaselineFlamegraphGenerator, Config as FlamegraphConfig, Flamegraph, FlamegraphGenerator,
    LoadBaselineFlamegraphGenerator, SaveBaselineFlamegraphGenerator,
};
use crate::runner::callgrind::parser::Sentinel;
use crate::runner::common::{Baselines, Config, ModulePath, Sandbox};
use crate::runner::format::{print_no_capture_footer, Formatter, OutputFormat, VerticalFormatter};
use crate::runner::meta::Metadata;
use crate::runner::summary::{
    BaselineKind, BaselineName, BenchmarkSummary, Profile, ProfileData, ProfileTotal,
    ToolMetricSummary, ToolRegression,
};
use crate::runner::{cachegrind, callgrind, DEFAULT_TOGGLE};
use crate::util::Glob;

/// The tool specific flamegraph configuration
#[derive(Debug, Clone, PartialEq)]
pub enum ToolFlamegraphConfig {
    /// The callgrind configuration
    Callgrind(FlamegraphConfig),
    /// If there is no configuration
    None,
}

/// The [`ToolConfig`] containing the basic configuration values to run the benchmark for this tool
#[derive(Debug, Clone)]
pub struct ToolConfig {
    /// The arguments to pass to the valgrind executable
    pub args: ToolArgs,
    /// The [`EntryPoint`] of this tool
    pub entry_point: EntryPoint,
    /// The tool specific flamegraph configuration
    pub flamegraph_config: ToolFlamegraphConfig,
    /// The [`Glob`] patterns used to matched a function in the call stack of a program point
    pub frames: Vec<Glob>,
    /// If true, this tool is the default tool for the benchmark run
    pub is_default: bool,
    /// If true, this tool is enabled for this benchmark
    pub is_enabled: bool,
    /// The tool specific regression check configuration
    pub regression_config: ToolRegressionConfig,
    /// The [`ValgrindTool`]
    pub tool: ValgrindTool,
}

/// Multiple [`ToolConfig`]s
#[derive(Debug, Clone)]
pub struct ToolConfigs(pub Vec<ToolConfig>);

impl ToolConfig {
    /// Create a new `ToolConfig`
    pub fn new<T>(
        tool: ValgrindTool,
        is_enabled: bool,
        args: T,
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
            regression_config,
            flamegraph_config,
            entry_point,
            is_default,
            frames: frames.iter().map(Into::into).collect(),
        })
    }

    /// Create a new `ToolConfig` from the given parameters
    #[allow(clippy::too_many_lines)]
    pub fn from_tool(
        output_format: &mut OutputFormat,
        valgrind_tool: ValgrindTool,
        tool: Option<Tool>,
        meta: &Metadata,
        base_args: &RawArgs,
        is_default: bool,
        regression_config: Option<ToolRegressionConfig>,
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
                .map(Ok)
                .or_else(|| tool.regression_config.map(TryInto::try_into))
                .transpose()
                .map_err(|error| anyhow!("Invalid limits for {valgrind_tool}: {error}"))?
                .unwrap_or(ToolRegressionConfig::None);
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

            Self::new(
                valgrind_tool,
                is_default || tool.enable.unwrap_or(true),
                args,
                regression_config,
                tool.flamegraph_config
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

            let mut regression_config = regression_config.unwrap_or(ToolRegressionConfig::None);
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

            Self::new(
                valgrind_tool,
                true,
                args,
                regression_config,
                ToolFlamegraphConfig::None,
                entry_point.unwrap_or(EntryPoint::None),
                is_default,
                &[],
            )
        }
    }

    /// Create a new default tool configuration
    #[allow(clippy::too_many_lines)]
    pub fn new_default_config(
        output_format: &mut OutputFormat,
        module_path: &ModulePath,
        id: Option<&String>,
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

                Self::from_tool(
                    output_format,
                    default_tool,
                    tool,
                    meta,
                    &base_args,
                    true,
                    meta.args.callgrind_limits.clone(),
                    Some(entry_point),
                )
            }
            ValgrindTool::Cachegrind => {
                let mut base_args = default_args
                    .get(&ValgrindTool::Cachegrind)
                    .cloned()
                    .unwrap_or_default();
                base_args.update(valgrind_args);

                Self::from_tool(
                    output_format,
                    ValgrindTool::Cachegrind,
                    tool,
                    meta,
                    &base_args,
                    true,
                    meta.args.cachegrind_limits.clone(),
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

                if entry_point == EntryPoint::Default {
                    let tool = tool.get_or_insert_with(|| Tool::new(ValgrindTool::DHAT));
                    let frames = tool.frames.get_or_insert_with(Vec::new);

                    // DHAT does not resolve function calls the same way as callgrind does. Somehow
                    // the benchmark function matched by the `DEFAULT_TOGGLE` gets sometimes inlined
                    // (although annotated with `#[inline(never)]`), so we need to fall back to the
                    // next best thing which is the function that calls the benchmark function. At
                    // this point the module path consists of `file::group::function`. The group in
                    // the path is artificial and we need the real function path within the
                    // benchmark file to create a matching glob pattern. That real path consists of
                    // `file::module::id`. The `id`-function won't be matched literally but with a
                    // wildcard to address the problem of functions with the same body being
                    // condensed into a single function by the compiler. Since in rare cases that
                    // can happen across modules the `module` is matched with a glob, too.
                    if let [first, _, last] = module_path.components()[..] {
                        frames.push(format!("{first}::{last}::*"));
                        if let Some(id) = id {
                            frames.push(format!("{first}::*::{id}"));
                        }
                    }
                }

                Self::from_tool(
                    output_format,
                    ValgrindTool::DHAT,
                    tool,
                    meta,
                    &base_args,
                    true,
                    meta.args.dhat_limits.clone(),
                    Some(entry_point),
                )
            }
            valgrind_tool => {
                let mut base_args = default_args
                    .get(&valgrind_tool)
                    .cloned()
                    .unwrap_or_default();
                base_args.update(valgrind_args);

                Self::from_tool(
                    output_format,
                    valgrind_tool,
                    tool,
                    meta,
                    &base_args,
                    true,
                    None,
                    None,
                )
            }
        }
    }

    /// Parse the [`Profile`] from profile data or log files
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
    #[allow(clippy::too_many_lines)]
    pub fn new(
        output_format: &mut OutputFormat,
        mut tools: Tools,
        module_path: &ModulePath,
        id: Option<&String>,
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
            id,
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

        let mut tool_configs = Self(vec![default_tool_config]);
        tool_configs.extend(meta_tools.into_iter().map(|mut tool| {
            let mut base_args = default_args.get(&tool.kind).cloned().unwrap_or_default();
            base_args.update(valgrind_args);

            match tool.kind {
                ValgrindTool::Callgrind => {
                    let entry_point = tool
                        .entry_point
                        .clone()
                        .unwrap_or_else(|| default_entry_point.clone());

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
                ),
                ValgrindTool::DHAT => {
                    let entry_point = tool
                        .entry_point
                        .clone()
                        .unwrap_or_else(|| default_entry_point.clone());

                    if entry_point == EntryPoint::Default {
                        let frames = tool.frames.get_or_insert_with(Vec::new);

                        // For the details see comment in `ToolConfig::new_default_config`
                        if let [first, _, last] = module_path.components()[..] {
                            frames.push(format!("{first}::{last}::*"));
                            if let Some(id) = id {
                                frames.push(format!("{first}::*::{id}"));
                            }
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
                ),
            }
        }))?;

        Ok(tool_configs)
    }

    /// Return true if there are any [`Tool`]s enabled
    pub fn has_tools_enabled(&self) -> bool {
        self.0.iter().any(|t| t.is_enabled)
    }

    /// Return true if there are multiple tools configured and are enabled
    pub fn has_multiple(&self) -> bool {
        self.0.len() > 1 && self.0.iter().filter(|f| f.is_enabled).count() > 1
    }

    /// Return all [`ToolOutputPath`]s of all enabled tools
    pub fn output_paths(&self, output_path: &ToolOutputPath) -> Vec<ToolOutputPath> {
        self.0
            .iter()
            .filter(|t| t.is_enabled)
            .map(|t| output_path.to_tool_output(t.tool))
            .collect()
    }

    /// Extend this collection of tools with the contents of an iterator
    pub fn extend<I>(&mut self, iter: I) -> Result<()>
    where
        I: Iterator<Item = Result<ToolConfig>>,
    {
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

    /// Run a benchmark when --load-baseline was given
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

    /// Run a benchmark with this configuration if not --load-baseline was given
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

impl From<Option<FlamegraphConfig>> for ToolFlamegraphConfig {
    fn from(value: Option<FlamegraphConfig>) -> Self {
        match value {
            Some(config) => Self::Callgrind(config),
            None => Self::None,
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
