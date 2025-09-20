//! The module containing the [`ToolConfig`] and other related elements

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::stderr;
use std::path::Path;

use anyhow::{anyhow, Result};

use super::args::ToolArgs;
use super::parser::{parser_factory, ParserOutput};
use super::path::ToolOutputPath;
use super::regression::{RegressionConfig, ToolRegressionConfig};
use super::run::{RunOptions, ToolCommand};
use crate::api::{self, EntryPoint, RawArgs, Tool, Tools, ValgrindTool};
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
    /// The wildcard patterns used to matched a function in the call stack of a program point
    pub frames: Vec<String>,
    /// If true, this tool is the default tool for the benchmark run
    pub is_default: bool,
    /// If true, this tool is enabled for this benchmark
    pub is_enabled: bool,
    /// The tool specific regression check configuration
    pub regression_config: ToolRegressionConfig,
    /// The [`ValgrindTool`]
    pub tool: ValgrindTool,
}

#[derive(Debug)]
struct ToolConfigBuilder {
    entry_point: Option<EntryPoint>,
    flamegraph_config: ToolFlamegraphConfig,
    frames: Vec<String>,
    is_default: bool,
    is_enabled: bool,
    kind: ValgrindTool,
    raw_args: RawArgs,
    regression_config: ToolRegressionConfig,
    tool: Option<Tool>,
}

/// Multiple [`ToolConfig`]s
#[derive(Debug, Clone)]
pub struct ToolConfigs(pub Vec<ToolConfig>);

impl ToolConfig {
    /// Create a new `ToolConfig`
    pub fn new(
        tool: ValgrindTool,
        is_enabled: bool,
        args: ToolArgs,
        regression_config: ToolRegressionConfig,
        flamegraph_config: ToolFlamegraphConfig,
        entry_point: EntryPoint,
        is_default: bool,
        frames: Vec<String>,
    ) -> Self {
        Self {
            args,
            entry_point,
            flamegraph_config,
            frames,
            is_default,
            is_enabled,
            regression_config,
            tool,
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

impl ToolConfigBuilder {
    fn build(self) -> Result<ToolConfig> {
        let args = match self.kind {
            ValgrindTool::Callgrind => {
                callgrind::args::Args::try_from_raw_args(&[&self.raw_args])?.into()
            }
            ValgrindTool::Cachegrind => {
                cachegrind::args::Args::try_from_raw_args(&[&self.raw_args])?.into()
            }
            _ => ToolArgs::try_from_raw_args(self.kind, &[&self.raw_args])?,
        };

        Ok(ToolConfig::new(
            self.kind,
            self.is_enabled,
            args,
            self.regression_config,
            self.flamegraph_config,
            self.entry_point.unwrap_or(EntryPoint::None),
            self.is_default,
            self.frames.iter().map(Into::into).collect(),
        ))
    }

    /// Build the entry point
    ///
    /// The `default_entry_point` can be different for example for binary benchmarks and library
    /// benchmarks.
    fn entry_point(
        &mut self,
        default_entry_point: &EntryPoint,
        module_path: &ModulePath,
        id: Option<&String>,
    ) {
        match self.kind {
            ValgrindTool::Callgrind => {
                let entry_point = self
                    .tool
                    .as_ref()
                    .and_then(|t| t.entry_point.clone())
                    .unwrap_or_else(|| default_entry_point.clone());

                match &entry_point {
                    EntryPoint::None => {}
                    EntryPoint::Default => {
                        self.raw_args
                            .extend_ignore_flag(&[format!("toggle-collect={DEFAULT_TOGGLE}")]);
                    }
                    EntryPoint::Custom(custom) => {
                        self.raw_args
                            .extend_ignore_flag(&[format!("toggle-collect={custom}")]);
                    }
                }

                self.entry_point = Some(entry_point);
            }
            ValgrindTool::DHAT => {
                let entry_point = self
                    .tool
                    .as_ref()
                    .and_then(|t| t.entry_point.clone())
                    .unwrap_or_else(|| default_entry_point.clone());

                if entry_point == EntryPoint::Default {
                    let mut frames = if let Some(tool) = self.tool.as_ref() {
                        if let Some(frames) = &tool.frames {
                            frames.clone()
                        } else {
                            Vec::default()
                        }
                    } else {
                        Vec::default()
                    };

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

                    self.frames = frames;
                }

                self.entry_point = Some(entry_point);
            }
            ValgrindTool::Cachegrind
            | ValgrindTool::Memcheck
            | ValgrindTool::Helgrind
            | ValgrindTool::DRD
            | ValgrindTool::Massif
            | ValgrindTool::BBV => {}
        }
    }

    fn flamegraph_config(&mut self) {
        if let Some(tool) = &self.tool {
            if let Some(flamegraph_config) = &tool.flamegraph_config {
                self.flamegraph_config = flamegraph_config.clone().into();
            }
        }
    }

    fn meta_args(&mut self, meta: &Metadata) {
        let raw_args = match self.kind {
            ValgrindTool::Callgrind => &meta.args.callgrind_args,
            ValgrindTool::Cachegrind => &meta.args.cachegrind_args,
            ValgrindTool::DHAT => &meta.args.dhat_args,
            ValgrindTool::Memcheck => &meta.args.memcheck_args,
            ValgrindTool::Helgrind => &meta.args.helgrind_args,
            ValgrindTool::DRD => &meta.args.drd_args,
            ValgrindTool::Massif => &meta.args.massif_args,
            ValgrindTool::BBV => &meta.args.bbv_args,
        };

        if let Some(args) = raw_args {
            self.raw_args.update(args);
        }
    }

    fn new(
        valgrind_tool: ValgrindTool,
        tool: Option<Tool>,
        is_default: bool,
        default_args: &HashMap<ValgrindTool, RawArgs>,
        module_path: &ModulePath,
        id: Option<&String>,
        meta: &Metadata,
        valgrind_args: &RawArgs,
        default_entry_point: &EntryPoint,
    ) -> Result<Self> {
        let mut builder = Self {
            is_enabled: is_default || tool.as_ref().map_or(true, |t| t.enable.unwrap_or(true)),
            tool,
            entry_point: Option::default(),
            flamegraph_config: ToolFlamegraphConfig::None,
            frames: Vec::default(),
            is_default,
            raw_args: default_args
                .get(&valgrind_tool)
                .cloned()
                .unwrap_or_default(),
            regression_config: ToolRegressionConfig::None,
            kind: valgrind_tool,
        };

        // Since the construction sequence is currently always the same, the construction of the
        // `ToolConfig` can happen here in one go instead of having a separate director for it.
        builder.valgrind_args(valgrind_args);
        builder.entry_point(default_entry_point, module_path, id);
        builder.tool_args();
        builder.meta_args(meta);
        builder.flamegraph_config();
        builder.regression_config(meta)?;

        Ok(builder)
    }

    fn regression_config(&mut self, meta: &Metadata) -> Result<()> {
        let meta_limits = match self.kind {
            ValgrindTool::Callgrind => meta.args.callgrind_limits.clone(),
            ValgrindTool::Cachegrind => meta.args.cachegrind_limits.clone(),
            ValgrindTool::DHAT => meta.args.dhat_limits.clone(),
            _ => None,
        };

        let mut regression_config = if let Some(tool) = &self.tool {
            meta_limits
                .map(Ok)
                .or_else(|| tool.regression_config.clone().map(TryInto::try_into))
                .transpose()
                .map_err(|error| anyhow!("Invalid limits for {}: {error}", self.kind))?
                .unwrap_or(ToolRegressionConfig::None)
        } else {
            meta_limits.unwrap_or(ToolRegressionConfig::None)
        };

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

        self.regression_config = regression_config;

        Ok(())
    }

    fn tool_args(&mut self) {
        if let Some(tool) = self.tool.as_ref() {
            self.raw_args.update(&tool.raw_args);
        }
    }

    fn valgrind_args(&mut self, valgrind_args: &RawArgs) {
        self.raw_args.update(valgrind_args);
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
    /// `valgrind_args` are from the in-benchmark configuration: `LibraryBenchmarkConfig` or
    /// `BinaryBenchmarkConfig`
    ///
    /// # Errors
    ///
    /// This function will return an error if the configs cannot be created
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

        output_format.update(extracted_tool.as_ref());
        let default_tool_config = ToolConfigBuilder::new(
            default_tool,
            extracted_tool,
            true,
            default_args,
            module_path,
            id,
            meta,
            valgrind_args,
            default_entry_point,
        )?
        .build()?;

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
        tool_configs.extend(meta_tools.into_iter().map(|tool| {
            output_format.update(Some(&tool));

            ToolConfigBuilder::new(
                tool.kind,
                Some(tool),
                false,
                default_args,
                module_path,
                id,
                meta,
                valgrind_args,
                default_entry_point,
            )?
            .build()
        }))?;

        output_format.update_from_meta(meta);
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
            && !output_format.show_only_comparison
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
    #[allow(clippy::too_many_lines)]
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
                if let Some(path) = output_path.to_xtree_output() {
                    path.clear()?;
                }
                if let Some(path) = output_path.to_xleak_output() {
                    path.clear()?;
                }
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
