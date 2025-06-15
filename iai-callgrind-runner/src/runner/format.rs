use std::borrow::Cow;
use std::fmt::{Display, Write};
use std::path::PathBuf;

use anyhow::Result;
use colored::{Color, ColoredString, Colorize};
use indexmap::{indexset, IndexSet};

use super::args::NoCapture;
use super::bin_bench::BinBench;
use super::common::{Baselines, BenchmarkSummaries, Config, ModulePath};
use super::lib_bench::LibBench;
use super::meta::Metadata;
use super::summary::{Diffs, MetricsDiff, ProfileData, ProfileInfo, ToolMetricSummary};
use crate::api::{
    self, CachegrindMetric, CallgrindMetrics, DhatMetric, ErrorMetric, EventKind, ValgrindTool,
};
use crate::runner::summary::MetricKind;
use crate::util::{
    make_relative, to_string_signed_short, to_string_unsigned_short, truncate_str_utf8,
    EitherOrBoth,
};

/// The string used to signal that a value is not available
pub const NOT_AVAILABLE: &str = "N/A";
pub const UNKNOWN: &str = "*********";
pub const NO_CHANGE: &str = "No change";

pub const METRIC_WIDTH: usize = 20;
pub const FIELD_WIDTH: usize = 21;

pub const LEFT_WIDTH: usize = METRIC_WIDTH + FIELD_WIDTH;
pub const DIFF_WIDTH: usize = 9;

/// The `DIFF_WIDTH` - the length of the unit
pub const FLOAT_WIDTH: usize = DIFF_WIDTH - 1;

#[allow(clippy::doc_link_with_quotes)]
/// The maximum line width
///
/// indent + left + "|" + metric width + " " + "(" + percentage + ")" + " " + "[" + factor + "]"
pub const MAX_WIDTH: usize = 2 + LEFT_WIDTH + 1 + METRIC_WIDTH + 2 * 11;

pub trait Formatter {
    fn format_single(
        &mut self,
        tool: ValgrindTool,
        baselines: &Baselines,
        info: Option<&EitherOrBoth<ProfileInfo>>,
        metrics_summary: &ToolMetricSummary,
        is_default_tool: bool,
    ) -> Result<()>;

    fn format(
        &mut self,
        tool: ValgrindTool,
        config: &Config,
        baselines: &Baselines,
        data: &ProfileData,
        is_default_tool: bool,
    ) -> Result<()>;

    fn format_free_form(&mut self, line: &str) -> Result<()>;

    fn print(
        &mut self,
        tool: ValgrindTool,
        config: &Config,
        baselines: &Baselines,
        data: &ProfileData,
        is_default_tool: bool,
    ) -> Result<()>
    where
        Self: std::fmt::Display,
    {
        if self.get_output_format().is_default() {
            self.format(tool, config, baselines, data, is_default_tool)?;
            print!("{self}");
            self.clear();
        }
        Ok(())
    }

    fn get_output_format(&self) -> &OutputFormat;

    fn clear(&mut self);

    fn print_comparison(
        &mut self,
        function_name: &str,
        id: &str,
        details: Option<&str>,
        summaries: Vec<(ValgrindTool, ToolMetricSummary)>,
    ) -> Result<()>;
}

pub struct BinaryBenchmarkHeader {
    inner: Header,
    output_format: OutputFormat,
}

pub struct ComparisonHeader {
    pub function_name: String,
    pub id: String,
    pub details: Option<String>,
    pub indent: String,
}

struct Header {
    module_path: String,
    id: Option<String>,
    description: Option<String>,
}

enum IndentKind {
    Normal,
    ToolHeadline,
    ToolSubHeadline,
}

pub struct LibraryBenchmarkHeader {
    inner: Header,
    output_format: OutputFormat,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormatKind {
    #[default]
    Default,
    Json,
    PrettyJson,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFormat {
    pub kind: OutputFormatKind,
    pub truncate_description: Option<usize>,
    pub show_intermediate: bool,
    pub show_grid: bool,
    pub callgrind: IndexSet<EventKind>,
    pub cachegrind: IndexSet<CachegrindMetric>,
    pub dhat: IndexSet<DhatMetric>,
    pub memcheck: IndexSet<ErrorMetric>,
    pub helgrind: IndexSet<ErrorMetric>,
    pub drd: IndexSet<ErrorMetric>,
}

#[derive(Debug, Clone)]
pub struct SummaryFormatter {
    pub output_format_kind: OutputFormatKind,
}

#[derive(Debug, Clone)]
pub struct VerticalFormatter {
    buffer: String,
    indent: String,
    indent_tool_header: String,
    indent_sub_header: String,
    output_format: OutputFormat,
}

impl BinaryBenchmarkHeader {
    pub fn new(meta: &Metadata, bin_bench: &BinBench) -> Self {
        let path = make_relative(&meta.project_root, &bin_bench.command.path);

        let command_args: Vec<String> = bin_bench
            .command
            .args
            .iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();
        let command_args = shlex::try_join(command_args.iter().map(String::as_str)).unwrap();

        let description = if command_args.is_empty() {
            format!(
                "({}) -> {}",
                bin_bench.args.as_ref().map_or("", String::as_str),
                path.display(),
            )
        } else {
            format!(
                "({}) -> {} {}",
                bin_bench.args.as_ref().map_or("", String::as_str),
                path.display(),
                command_args
            )
        };

        Self {
            inner: Header::new(
                &bin_bench.module_path,
                bin_bench.id.clone(),
                Some(description),
                &bin_bench.output_format,
            ),
            output_format: bin_bench.output_format.clone(),
        }
    }

    pub fn print(&self) {
        if self.output_format.kind == OutputFormatKind::Default {
            self.inner.print();
        }
    }

    pub fn to_title(&self) -> String {
        self.inner.to_title()
    }

    pub fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }
}

impl ComparisonHeader {
    pub fn new<T, U, V>(
        function_name: T,
        id: U,
        details: Option<V>,
        output_format: &OutputFormat,
    ) -> Self
    where
        T: Into<String>,
        U: Into<String>,
        V: Into<String>,
    {
        Self {
            function_name: function_name.into(),
            id: id.into(),
            details: details.map(Into::into),
            indent: if output_format.show_grid {
                "|-".bright_black().to_string()
            } else {
                "  ".to_owned()
            },
        }
    }

    pub fn print(&self) {
        println!("{self}");
    }
}

impl Display for ComparisonHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{} {} {}",
            self.indent,
            "Comparison with".yellow().bold(),
            self.function_name.green(),
            self.id.cyan()
        )?;

        if let Some(details) = &self.details {
            write!(f, ":{}", details.blue().bold())?;
        }

        Ok(())
    }
}

impl Header {
    pub fn new<T>(
        module_path: &ModulePath,
        id: T,
        description: Option<String>,
        output_format: &OutputFormat,
    ) -> Self
    where
        T: Into<Option<String>>,
    {
        let truncated = description
            .map(|d| truncate_description(&d, output_format.truncate_description).to_string());

        Self {
            module_path: module_path.to_string(),
            id: id.into(),
            description: truncated,
        }
    }

    pub fn print(&self) {
        println!("{self}");
    }

    pub fn to_title(&self) -> String {
        let mut output = String::new();

        write!(output, "{}", self.module_path).unwrap();
        if let Some(id) = &self.id {
            match &self.description {
                Some(description) if !description.is_empty() => {
                    write!(output, " {id}:{description}").unwrap();
                }
                _ => {
                    write!(output, " {id}").unwrap();
                }
            }
        }
        output
    }
}

impl Display for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.module_path.green()))?;

        if let Some(id) = &self.id {
            match &self.description {
                Some(description) if !description.is_empty() => {
                    f.write_fmt(format_args!(
                        " {}{}{}",
                        id.cyan(),
                        ":".cyan(),
                        description.bold().blue(),
                    ))?;
                }
                _ if !id.is_empty() => {
                    f.write_fmt(format_args!(" {}", id.cyan()))?;
                }
                _ => {}
            }
        } else if let Some(description) = &self.description {
            if !description.is_empty() {
                f.write_fmt(format_args!(" {}", description.bold().blue()))?;
            }
        } else {
            // do nothing
        }
        Ok(())
    }
}

impl LibraryBenchmarkHeader {
    pub fn new(lib_bench: &LibBench) -> Self {
        let header = Header::new(
            &lib_bench.module_path,
            lib_bench.id.clone(),
            lib_bench.args.clone(),
            &lib_bench.output_format,
        );

        Self {
            inner: header,
            output_format: lib_bench.output_format.clone(),
        }
    }

    pub fn print(&self) {
        if self.output_format.is_default() {
            self.inner.print();
        }
    }

    pub fn to_title(&self) -> String {
        self.inner.to_title()
    }

    pub fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }
}

impl OutputFormat {
    pub fn is_default(&self) -> bool {
        self.kind == OutputFormatKind::Default
    }

    pub fn is_json(&self) -> bool {
        self.kind == OutputFormatKind::Json || self.kind == OutputFormatKind::PrettyJson
    }
}

impl From<api::OutputFormat> for OutputFormat {
    fn from(value: api::OutputFormat) -> Self {
        Self {
            kind: OutputFormatKind::Default,
            truncate_description: value.truncate_description.unwrap_or(Some(50)),
            show_intermediate: value.show_intermediate.unwrap_or(false),
            show_grid: value.show_grid.unwrap_or(false),
            ..Default::default()
        }
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self {
            kind: OutputFormatKind::default(),
            truncate_description: Some(50),
            show_intermediate: false,
            show_grid: false,
            callgrind: IndexSet::from(CallgrindMetrics::Default),
            cachegrind: indexset![
                CachegrindMetric::Ir,
                CachegrindMetric::L1hits,
                CachegrindMetric::LLhits,
                CachegrindMetric::RamHits,
                CachegrindMetric::TotalRW,
                CachegrindMetric::EstimatedCycles,
                CachegrindMetric::Bc,
                CachegrindMetric::Bcm,
                CachegrindMetric::Bi,
                CachegrindMetric::Bim,
            ],
            dhat: indexset![
                DhatMetric::TotalBytes,
                DhatMetric::TotalBlocks,
                DhatMetric::AtTGmaxBytes,
                DhatMetric::AtTGmaxBlocks,
                DhatMetric::AtTEndBytes,
                DhatMetric::AtTEndBlocks,
                DhatMetric::ReadsBytes,
                DhatMetric::WritesBytes,
            ],
            memcheck: indexset![
                ErrorMetric::Errors,
                ErrorMetric::Contexts,
                ErrorMetric::SuppressedErrors,
                ErrorMetric::SuppressedContexts,
            ],
            helgrind: indexset![
                ErrorMetric::Errors,
                ErrorMetric::Contexts,
                ErrorMetric::SuppressedErrors,
                ErrorMetric::SuppressedContexts,
            ],
            drd: indexset![
                ErrorMetric::Errors,
                ErrorMetric::Contexts,
                ErrorMetric::SuppressedErrors,
                ErrorMetric::SuppressedContexts,
            ],
        }
    }
}

impl SummaryFormatter {
    pub fn new(output_format_kind: OutputFormatKind) -> Self {
        Self { output_format_kind }
    }

    pub fn print(&self, summaries: &BenchmarkSummaries) {
        if self.output_format_kind == OutputFormatKind::Default {
            let total_benchmarks = summaries.num_benchmarks();
            let total_time = to_string_unsigned_short(
                summaries
                    .total_time
                    .expect("The total execution time should be present")
                    .as_secs_f64(),
            );

            if summaries.is_regressed() {
                println!("\nRegressions:\n");
                let mut num_regressed = 0;
                for summary in summaries.summaries.iter().filter(|p| p.is_regressed()) {
                    if let Some(id) = &summary.id {
                        println!("  {} {}:", summary.module_path.green(), id.cyan());
                    } else {
                        println!("  {}:", summary.module_path.green());
                    }
                    for regression in summary
                        .profiles
                        .iter()
                        .flat_map(|t| &t.summaries.total.regressions)
                    {
                        let metric_string = if let MetricKind::Callgrind(event) = regression.metric
                        {
                            event.to_string()
                        } else {
                            regression.metric.to_string()
                        };
                        println!(
                            "    {} ({} -> {}): {:>6}{} exceeds limit of {:>6}{}",
                            metric_string.bold(),
                            regression.old,
                            regression.new.to_string().bold(),
                            to_string_signed_short(regression.diff_pct)
                                .bright_red()
                                .bold(),
                            "%".bright_red().bold(),
                            to_string_signed_short(regression.limit).bright_black(),
                            "%".bright_black()
                        );
                    }

                    num_regressed += 1;
                }

                let num_not_regressed = total_benchmarks - num_regressed;
                println!(
                    "\nIai-Callgrind result: {}. {num_not_regressed} without regressions; \
                     {num_regressed} regressed; {total_benchmarks} benchmarks finished in \
                     {total_time:>6}s",
                    "Regressed".bright_red().bold(),
                );
            } else {
                println!(
                    "\nIai-Callgrind result: {}. {total_benchmarks} without regressions; 0 \
                     regressed; {total_benchmarks} benchmarks finished in {total_time:>6}s",
                    "Ok".green().bold(),
                );
            }
        }
    }
}

impl VerticalFormatter {
    /// Create a new `VerticalFormatter` (the default format)
    pub fn new(output_format: OutputFormat) -> Self {
        if output_format.show_grid {
            Self {
                buffer: String::new(),
                indent: "| ".bright_black().to_string(),
                indent_sub_header: "|-".bright_black().to_string(),
                indent_tool_header: "|=".bright_black().to_string(),
                output_format,
            }
        } else {
            Self {
                buffer: String::new(),
                indent: "  ".bright_black().to_string(),
                indent_sub_header: "  ".bright_black().to_string(),
                indent_tool_header: "  ".bright_black().to_string(),
                output_format,
            }
        }
    }

    /// Print the internal buffer as is and clear it afterwards
    pub fn print_buffer(&mut self) {
        print!("{}", self.buffer);
        self.clear();
    }

    /// Write the indentation depending on the chosen [`OutputFormat`] and [`IndentKind`]
    fn write_indent(&mut self, kind: &IndentKind) {
        match kind {
            IndentKind::Normal => write!(self, "{}", self.indent.clone()).unwrap(),
            IndentKind::ToolHeadline => {
                write!(self, "{}", self.indent_tool_header.clone()).unwrap();
            }
            IndentKind::ToolSubHeadline => {
                write!(self, "{}", self.indent_sub_header.clone()).unwrap();
            }
        }
    }

    fn write_field<T>(
        &mut self,
        field: &str,
        values: &EitherOrBoth<T>,
        color: Option<Color>,
        left_align: bool,
    ) where
        T: AsRef<str>,
    {
        self.write_indent(&IndentKind::Normal);

        match values {
            EitherOrBoth::Left(left) => {
                let left = left.as_ref();
                let colored = match color {
                    Some(color) => left.color(color).bold(),
                    None => left.bold(),
                };

                if left_align {
                    writeln!(self, "{field:<FIELD_WIDTH$}{colored}").unwrap();
                } else {
                    writeln!(
                        self,
                        "{field:<FIELD_WIDTH$}{}{colored}",
                        " ".repeat(METRIC_WIDTH.saturating_sub(left.len()))
                    )
                    .unwrap();
                }
            }
            EitherOrBoth::Right(right) => {
                let right = right.as_ref().trim();
                let colored = match color {
                    Some(color) => right.color(color),
                    None => ColoredString::from(right),
                };

                writeln!(
                    self,
                    "{field:<FIELD_WIDTH$}{}|{colored}",
                    " ".repeat(METRIC_WIDTH),
                )
                .unwrap();
            }
            EitherOrBoth::Both(left, right) => {
                let left = left.as_ref().trim();
                let right = right.as_ref().trim();

                let colored_left = match color {
                    Some(color) => left.color(color).bold(),
                    None => left.bold(),
                };
                let colored_right = match color {
                    Some(color) => right.color(color),
                    None => ColoredString::from(right),
                };

                if left.len() > METRIC_WIDTH {
                    writeln!(self, "{field:<FIELD_WIDTH$}{colored_left}").unwrap();
                    self.write_indent(&IndentKind::Normal);
                    writeln!(self, "{}|{colored_right}", " ".repeat(LEFT_WIDTH)).unwrap();
                } else if left_align {
                    writeln!(
                        self,
                        "{field:<FIELD_WIDTH$}{colored_left}{}|{colored_right}",
                        " ".repeat(METRIC_WIDTH - left.len()),
                    )
                    .unwrap();
                } else {
                    writeln!(
                        self,
                        "{field:<FIELD_WIDTH$}{}{colored_left}|{colored_right}",
                        " ".repeat(METRIC_WIDTH - left.len()),
                    )
                    .unwrap();
                }
            }
        }
    }

    fn write_metric(&mut self, field: &str, metrics: &EitherOrBoth<&u64>, diffs: Option<Diffs>) {
        match metrics {
            EitherOrBoth::Left(new) => {
                let right = format!(
                    "{NOT_AVAILABLE:<METRIC_WIDTH$} ({:^DIFF_WIDTH$})",
                    UNKNOWN.bright_black()
                );
                self.write_field(
                    field,
                    &EitherOrBoth::Both(&new.to_string(), &right),
                    None,
                    false,
                );
            }
            EitherOrBoth::Right(old) => {
                let right = format!(
                    "{old:<METRIC_WIDTH$} ({:^DIFF_WIDTH$})",
                    UNKNOWN.bright_black()
                );
                self.write_field(
                    field,
                    &EitherOrBoth::Both(NOT_AVAILABLE, &right),
                    None,
                    false,
                );
            }
            EitherOrBoth::Both(new, old) if new == old => {
                let right = format!(
                    "{old:<METRIC_WIDTH$} ({:^DIFF_WIDTH$})",
                    NO_CHANGE.bright_black()
                );
                self.write_field(
                    field,
                    &EitherOrBoth::Both(&new.to_string(), &right),
                    None,
                    false,
                );
            }
            EitherOrBoth::Both(new, old) => {
                let diffs = diffs.expect(
                    "If there are new metrics and old metrics there should be a difference present",
                );
                let pct_string = format_float(diffs.diff_pct, '%');
                let factor_string = format_float(diffs.factor, 'x');

                let right = format!(
                    "{old:<METRIC_WIDTH$} ({pct_string:^DIFF_WIDTH$}) \
                     [{factor_string:^DIFF_WIDTH$}]"
                );
                self.write_field(
                    field,
                    &EitherOrBoth::Both(&new.to_string(), &right),
                    None,
                    false,
                );
            }
        }
    }

    fn write_empty_line(&mut self) {
        let indent = self.indent.trim_end().to_owned();
        if !indent.is_empty() {
            writeln!(self, "{indent}").unwrap();
        }
    }

    fn write_left_indented(&mut self, value: &str) {
        self.write_indent(&IndentKind::Normal);
        writeln!(self, "{}{value}", " ".repeat(FIELD_WIDTH)).unwrap();
    }

    /// Format the baseline
    fn format_baseline(&mut self, baselines: &Baselines) {
        match baselines {
            (None, None) => {}
            _ => {
                self.write_field(
                    "Baselines:",
                    &EitherOrBoth::try_from(baselines.clone())
                        .expect("At least on baseline should be present")
                        .as_ref()
                        .map(String::as_str),
                    None,
                    false,
                );
            }
        }
    }

    fn format_details(&mut self, details: &str) {
        let mut details = details.lines();
        if let Some(head_line) = details.next() {
            self.write_indent(&IndentKind::Normal);
            writeln!(self, "{:<FIELD_WIDTH$}{}", "Details:", head_line).unwrap();
            for body_line in details {
                if body_line.is_empty() {
                    self.write_empty_line();
                } else {
                    self.write_left_indented(body_line);
                }
            }
        }
    }

    fn format_metrics<'a, K: Display>(
        &mut self,
        metrics: impl Iterator<Item = (K, &'a MetricsDiff)>,
    ) {
        for (metric_kind, diff) in metrics {
            let description = format!("{metric_kind}:");
            self.write_metric(&description, &diff.metrics.as_ref(), diff.diffs);
        }
    }

    fn format_tool_total_header(&mut self) {
        self.write_indent(&IndentKind::ToolSubHeadline);
        writeln!(self, "{} {}", "##".yellow(), "Total".bold()).unwrap();
    }

    fn format_multiple_segment_header(&mut self, details: &EitherOrBoth<ProfileInfo>) {
        fn fields(detail: &ProfileInfo) -> String {
            let mut result = String::new();
            write!(result, "pid: {}", detail.pid).unwrap();

            if let Some(ppid) = detail.parent_pid {
                write!(result, " ppid: {ppid}").unwrap();
            }
            if let Some(thread) = detail.thread {
                write!(result, " thread: {thread}").unwrap();
            }
            if let Some(part) = detail.part {
                write!(result, " part: {part}").unwrap();
            }

            result
        }

        self.write_indent(&IndentKind::ToolSubHeadline);
        write!(self, "{} ", "##".yellow()).unwrap();

        let max_left = LEFT_WIDTH - 3;
        match details {
            EitherOrBoth::Left(new) => {
                let left = fields(new);
                let len = left.len();
                let left = left.bold();

                if len > max_left {
                    writeln!(self, "{left}\n{}|{NOT_AVAILABLE}", " ".repeat(max_left + 5)).unwrap();
                } else {
                    writeln!(self, "{left}{}|{NOT_AVAILABLE}", " ".repeat(max_left - len)).unwrap();
                }
            }
            EitherOrBoth::Right(old) => {
                let right = fields(old);

                writeln!(
                    self,
                    "{}{}|{right}",
                    NOT_AVAILABLE.bold(),
                    " ".repeat(max_left - NOT_AVAILABLE.len())
                )
                .unwrap();
            }
            EitherOrBoth::Both(new, old) => {
                let left = fields(new);
                let len = left.len();
                let right = fields(old);
                let left = left.bold();

                if len > max_left {
                    writeln!(self, "{left}\n{}|{right}", " ".repeat(max_left + 5)).unwrap();
                } else {
                    writeln!(self, "{left}{}|{right}", " ".repeat(max_left - len)).unwrap();
                }
            }
        }
    }

    fn format_command(&mut self, config: &Config, command: &EitherOrBoth<&String>) {
        let paths = match command {
            EitherOrBoth::Left(new) => {
                if new.starts_with(&config.bench_bin.display().to_string()) {
                    EitherOrBoth::Left(make_relative(&config.meta.project_root, &config.bench_bin))
                } else {
                    EitherOrBoth::Left(make_relative(&config.meta.project_root, PathBuf::from(new)))
                }
            }
            EitherOrBoth::Right(old) => {
                if old.starts_with(&config.bench_bin.display().to_string()) {
                    EitherOrBoth::Right(make_relative(&config.meta.project_root, &config.bench_bin))
                } else {
                    EitherOrBoth::Right(make_relative(
                        &config.meta.project_root,
                        PathBuf::from(old),
                    ))
                }
            }
            EitherOrBoth::Both(new, old) if new == old => {
                if new.starts_with(&config.bench_bin.display().to_string()) {
                    EitherOrBoth::Left(make_relative(&config.meta.project_root, &config.bench_bin))
                } else {
                    EitherOrBoth::Left(make_relative(&config.meta.project_root, PathBuf::from(new)))
                }
            }
            EitherOrBoth::Both(new, old) => {
                let new_command = if new.starts_with(&config.bench_bin.display().to_string()) {
                    make_relative(&config.meta.project_root, &config.bench_bin)
                } else {
                    make_relative(&config.meta.project_root, PathBuf::from(new))
                };
                let old_command = if old.starts_with(&config.bench_bin.display().to_string()) {
                    make_relative(&config.meta.project_root, &config.bench_bin)
                } else {
                    make_relative(&config.meta.project_root, PathBuf::from(old))
                };
                EitherOrBoth::Both(new_command, old_command)
            }
        };

        self.write_field(
            "Command:",
            &paths.map(|p| p.display().to_string()),
            Some(Color::Blue),
            true,
        );
    }

    pub fn format_tool_headline(&mut self, tool: ValgrindTool) {
        self.write_indent(&IndentKind::ToolHeadline);

        let id = tool.id();
        writeln!(
            self,
            "{} {} {}",
            "=======".bright_black(),
            id.to_ascii_uppercase(),
            "=".repeat(MAX_WIDTH.saturating_sub(id.len() + 9))
                .bright_black(),
        )
        .unwrap();
    }
}

impl Display for VerticalFormatter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.buffer)
    }
}

impl Write for VerticalFormatter {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.buffer.push_str(s);
        Ok(())
    }
}

impl Formatter for VerticalFormatter {
    fn format_single(
        &mut self,
        tool: ValgrindTool,
        baselines: &Baselines,
        info: Option<&EitherOrBoth<ProfileInfo>>,
        metrics_summary: &ToolMetricSummary,
        is_default_tool: bool,
    ) -> Result<()> {
        if is_default_tool {
            self.format_baseline(baselines);
        }

        match metrics_summary {
            ToolMetricSummary::None => {
                if let Some(info) = info {
                    if let Some(new) = info.left() {
                        if let Some(details) = &new.details {
                            self.format_details(details);
                        }
                    }
                }
            }
            ToolMetricSummary::ErrorTool(summary) => {
                let format = match tool {
                    ValgrindTool::Memcheck => &self.output_format.memcheck,
                    ValgrindTool::Helgrind => &self.output_format.helgrind,
                    ValgrindTool::DRD => &self.output_format.drd,
                    _ => {
                        unreachable!("{tool} should be an error metric tool");
                    }
                };

                self.format_metrics(
                    format
                        .clone()
                        .iter()
                        .filter_map(|e| summary.diff_by_kind(e).map(|d| (e, d))),
                );

                // We only check for `new` errors
                if let Some(info) = info {
                    if summary
                        .diff_by_kind(&ErrorMetric::Errors)
                        .is_some_and(|e| e.metrics.left().is_some_and(|l| *l > 0))
                    {
                        if let Some(new) = info.left() {
                            if let Some(details) = new.details.as_ref() {
                                self.format_details(details);
                            }
                        }
                    }
                }
            }
            ToolMetricSummary::Dhat(summary) => self.format_metrics(
                self.output_format
                    .dhat
                    .clone()
                    .iter()
                    .filter_map(|e| summary.diff_by_kind(e).map(|d| (e, d))),
            ),
            ToolMetricSummary::Callgrind(summary) => {
                self.format_metrics(
                    self.output_format
                        .callgrind
                        .clone()
                        .iter()
                        .filter_map(|e| summary.diff_by_kind(e).map(|d| (e, d))),
                );
            }
            ToolMetricSummary::Cachegrind(summary) => {
                self.format_metrics(
                    self.output_format
                        .cachegrind
                        .clone()
                        .iter()
                        .filter_map(|e| summary.diff_by_kind(e).map(|d| (e, d))),
                );
            }
        }
        Ok(())
    }

    fn format(
        &mut self,
        tool: ValgrindTool,
        config: &Config,
        baselines: &Baselines,
        data: &ProfileData,
        is_default_tool: bool,
    ) -> Result<()> {
        if data.has_multiple() && self.output_format.show_intermediate {
            let mut first = true;
            for part in &data.parts {
                self.format_multiple_segment_header(&part.details);
                self.format_command(config, &part.details.as_ref().map(|i| &i.command));

                if first {
                    self.format_single(
                        tool,
                        baselines,
                        Some(&part.details),
                        &part.metrics_summary,
                        is_default_tool,
                    )?;
                    first = false;
                } else {
                    self.format_single(
                        tool,
                        &(None, None),
                        Some(&part.details),
                        &part.metrics_summary,
                        is_default_tool,
                    )?;
                }
            }

            if data.total.is_some() {
                self.format_tool_total_header();
                self.format_single(
                    tool,
                    &(None, None),
                    None,
                    &data.total.summary,
                    is_default_tool,
                )?;
            }
        } else if data.total.is_some() {
            self.format_single(tool, baselines, None, &data.total.summary, is_default_tool)?;
        } else if data.total.is_none() && !data.parts.is_empty() {
            // Since there is no total, show_all is partly ignored, and we show all data in a little
            // bit more aggregated form without the multiple files headlines. This affects currently
            // the output of `Massif` and `BBV`.
            for part in &data.parts {
                self.format_command(config, &part.details.as_ref().map(|i| &i.command));

                if let Some(new) = part.details.left() {
                    if let Some(details) = &new.details {
                        self.format_details(details);
                    }
                }
            }
        } else {
            // no data to show
        }

        Ok(())
    }

    fn print_comparison(
        &mut self,
        function_name: &str,
        id: &str,
        details: Option<&str>,
        summaries: Vec<(ValgrindTool, ToolMetricSummary)>,
    ) -> Result<()> {
        if self.output_format.is_default() {
            ComparisonHeader::new(function_name, id, details, &self.output_format).print();

            let is_multiple = summaries.len() > 1;
            for (tool, summary) in summaries {
                if is_multiple || tool != ValgrindTool::Callgrind {
                    self.format_free_form(&format!(
                        "{}{} {}\n",
                        self.indent_sub_header,
                        "-------".bright_black(),
                        tool.to_string().to_uppercase()
                    ))?;
                }
                self.format_single(tool, &(None, None), None, &summary, false)?;
            }
            self.print_buffer();
        }

        Ok(())
    }

    fn clear(&mut self) {
        self.buffer.clear();
    }

    fn get_output_format(&self) -> &OutputFormat {
        &self.output_format
    }

    fn format_free_form(&mut self, line: &str) -> Result<()> {
        self.buffer.push_str(line);
        Ok(())
    }
}

pub fn format_float(float: f64, unit: char) -> ColoredString {
    let signed_short = to_string_signed_short(float);
    if float.is_infinite() {
        if float.is_sign_positive() {
            format!("{signed_short:+^DIFF_WIDTH$}").bright_red().bold()
        } else {
            format!("{signed_short:-^DIFF_WIDTH$}")
                .bright_green()
                .bold()
        }
    } else if float.is_sign_positive() {
        format!("{signed_short:>+FLOAT_WIDTH$}{unit}")
            .bright_red()
            .bold()
    } else {
        format!("{signed_short:>+FLOAT_WIDTH$}{unit}")
            .bright_green()
            .bold()
    }
}

// Return the formatted `String` if `NoCapture` is not `False`
pub fn no_capture_footer(nocapture: NoCapture) -> Option<String> {
    match nocapture {
        NoCapture::True => Some(format!(
            "{} {}",
            "-".yellow(),
            "end of stdout/stderr".yellow()
        )),
        NoCapture::False => None,
        NoCapture::Stderr => Some(format!("{} {}", "-".yellow(), "end of stderr".yellow())),
        NoCapture::Stdout => Some(format!("{} {}", "-".yellow(), "end of stdout".yellow())),
    }
}

pub fn print_no_capture_footer(
    nocapture: NoCapture,
    stdout: Option<&api::Stdio>,
    stderr: Option<&api::Stdio>,
) {
    let stdout_is_pipe = stdout.map_or(
        nocapture == NoCapture::False || nocapture == NoCapture::Stderr,
        api::Stdio::is_pipe,
    );

    let stderr_is_pipe = stderr.map_or(
        nocapture == NoCapture::False || nocapture == NoCapture::Stdout,
        api::Stdio::is_pipe,
    );

    // These unwraps are safe because `no_capture_footer` returns None only if `NoCapture` is
    // `False`
    match (stdout_is_pipe, stderr_is_pipe) {
        (true, true) => {}
        (true, false) => {
            println!("{}", no_capture_footer(NoCapture::Stderr).unwrap());
        }
        (false, true) => {
            println!("{}", no_capture_footer(NoCapture::Stdout).unwrap());
        }
        (false, false) => {
            println!("{}", no_capture_footer(NoCapture::True).unwrap());
        }
    }
}

pub fn print_list_benchmark(module_path: &ModulePath, id: Option<&String>) {
    match id {
        Some(id) => {
            println!("{module_path}::{id}: benchmark");
        }
        None => {
            println!("{module_path}: benchmark");
        }
    }
}

pub fn print_benchmark_list_summary(sum: u64) {
    if sum != 0 {
        println!();
    }
    println!("0 tests, {sum} benchmarks");
}

fn truncate_description(description: &str, truncate_description: Option<usize>) -> Cow<'_, str> {
    if let Some(num) = truncate_description {
        let new_description = truncate_str_utf8(description, num);
        if new_description.len() < description.len() {
            Cow::Owned(format!("{new_description}..."))
        } else {
            Cow::Borrowed(description)
        }
    } else {
        Cow::Borrowed(description)
    }
}

#[cfg(test)]
mod tests {
    use indexmap::indexmap;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;
    use crate::runner::metrics::Metrics;

    #[rstest]
    #[case::simple("some::module", Some("id"), Some("1, 2"), "some::module id:1, 2")]
    #[case::id_but_no_description("some::module", Some("id"), None, "some::module id")]
    #[case::id_but_empty_description("some::module", Some("id"), Some(""), "some::module id")]
    #[case::no_id_but_description("some::module", None, Some("1, 2, 3"), "some::module 1, 2, 3")]
    #[case::no_id_no_description("some::module", None, None, "some::module")]
    #[case::no_id_empty_description("some::module", None, Some(""), "some::module")]
    #[case::length_is_greater_than_default(
        "some::module",
        Some("id"),
        Some("012345678901234567890123456789012345678901234567890123456789"),
        "some::module id:012345678901234567890123456789012345678901234567890123456789"
    )]
    fn test_header_display_when_no_truncate(
        #[case] module_path: &str,
        #[case] id: Option<&str>,
        #[case] description: Option<&str>,
        #[case] expected: &str,
    ) {
        colored::control::set_override(false);

        let output_format = OutputFormat {
            truncate_description: None,
            ..Default::default()
        };
        let header = Header::new(
            &ModulePath::new(module_path),
            id.map(ToOwned::to_owned),
            description.map(ToOwned::to_owned),
            &output_format,
        );

        assert_eq!(header.to_string(), expected);
    }

    #[rstest]
    #[case::truncate_0(
        "some::module",
        Some("id"),
        Some("1, 2, 3"),
        Some(0),
        "some::module id:..."
    )]
    #[case::truncate_0_when_length_is_0(
        "some::module",
        Some("id"),
        Some(""),
        Some(0),
        "some::module id"
    )]
    #[case::truncate_0_when_length_is_1(
        "some::module",
        Some("id"),
        Some("1"),
        Some(0),
        "some::module id:..."
    )]
    #[case::truncate_1(
        "some::module",
        Some("id"),
        Some("1, 2, 3"),
        Some(1),
        "some::module id:1..."
    )]
    #[case::truncate_1_when_length_is_0(
        "some::module",
        Some("id"),
        Some(""),
        Some(1),
        "some::module id"
    )]
    #[case::truncate_1_when_length_is_1(
        "some::module",
        Some("id"),
        Some("1"),
        Some(1),
        "some::module id:1"
    )]
    #[case::truncate_1_when_length_is_2(
        "some::module",
        Some("id"),
        Some("1,"),
        Some(1),
        "some::module id:1..."
    )]
    #[case::truncate_3(
        "some::module",
        Some("id"),
        Some("1, 2, 3"),
        Some(3),
        "some::module id:1, ..."
    )]
    #[case::truncate_3_when_length_is_2(
        "some::module",
        Some("id"),
        Some("1,"),
        Some(3),
        "some::module id:1,"
    )]
    #[case::truncate_3_when_length_is_3(
        "some::module",
        Some("id"),
        Some("1, "),
        Some(3),
        "some::module id:1, "
    )]
    #[case::truncate_3_when_length_is_4(
        "some::module",
        Some("id"),
        Some("1, 2"),
        Some(3),
        "some::module id:1, ..."
    )]
    #[case::truncate_is_smaller_than_length(
        "some::module",
        Some("id"),
        Some("1, 2, 3, 4, 5"),
        Some(4),
        "some::module id:1, 2..."
    )]
    #[case::truncate_is_one_smaller_than_length(
        "some::module",
        Some("id"),
        Some("1, 2, 3"),
        Some(6),
        "some::module id:1, 2, ..."
    )]
    #[case::truncate_is_one_greater_than_length(
        "some::module",
        Some("id"),
        Some("1, 2, 3"),
        Some(8),
        "some::module id:1, 2, 3"
    )]
    #[case::truncate_is_far_greater_than_length(
        "some::module",
        Some("id"),
        Some("1, 2, 3"),
        Some(100),
        "some::module id:1, 2, 3"
    )]
    #[case::truncate_is_equal_to_length(
        "some::module",
        Some("id"),
        Some("1, 2, 3"),
        Some(7),
        "some::module id:1, 2, 3"
    )]
    #[case::description_is_empty(
        "some::module",
        Some("id"),
        Some(""),
        Some(100),
        "some::module id"
    )]
    fn test_header_display_when_truncate(
        #[case] module_path: &str,
        #[case] id: Option<&str>,
        #[case] description: Option<&str>,
        #[case] truncate_description: Option<usize>,
        #[case] expected: &str,
    ) {
        colored::control::set_override(false);

        let output_format = OutputFormat {
            truncate_description,
            ..Default::default()
        };

        let header = Header::new(
            &ModulePath::new(module_path),
            id.map(ToOwned::to_owned),
            description.map(ToOwned::to_owned),
            &output_format,
        );

        assert_eq!(header.to_string(), expected);
    }

    #[rstest]
    #[case::new_costs_0(EventKind::Ir, 0, None, "*********", None)]
    #[case::old_costs_0(EventKind::Ir, 1, Some(0), "+++inf+++", Some("+++inf+++"))]
    #[case::all_costs_0(EventKind::Ir, 0, Some(0), "No change", None)]
    #[case::new_costs_u64_max(EventKind::Ir, u64::MAX, None, "*********", None)]
    #[case::old_costs_u64_max(EventKind::Ir, u64::MAX / 10, Some(u64::MAX), "-90.0000%", Some("-10.0000x"))]
    #[case::all_costs_u64_max(EventKind::Ir, u64::MAX, Some(u64::MAX), "No change", None)]
    #[case::no_change_when_not_0(EventKind::Ir, 1000, Some(1000), "No change", None)]
    #[case::neg_change_when_not_0(EventKind::Ir, 2000, Some(3000), "-33.3333%", Some("-1.50000x"))]
    #[case::pos_change_when_not_0(EventKind::Ir, 2000, Some(1000), "+100.000%", Some("+2.00000x"))]
    #[case::pos_inf(EventKind::Ir, 2000, Some(0), "+++inf+++", Some("+++inf+++"))]
    #[case::neg_inf(EventKind::Ir, 0, Some(2000), "-100.000%", Some("---inf---"))]
    fn test_format_vertical_when_new_costs_are_present(
        #[case] event_kind: EventKind,
        #[case] new: u64,
        #[case] old: Option<u64>,
        #[case] diff_pct: &str,
        #[case] diff_fact: Option<&str>,
    ) {
        use crate::runner::summary::MetricsSummary;

        colored::control::set_override(false);

        let costs = match old {
            Some(old) => EitherOrBoth::Both(
                Metrics(indexmap! {event_kind => new}),
                Metrics(indexmap! {event_kind => old}),
            ),
            None => EitherOrBoth::Left(Metrics(indexmap! {event_kind => new})),
        };
        let metrics_summary = MetricsSummary::new(costs);
        let mut formatter = VerticalFormatter::new(OutputFormat::default());
        formatter.format_metrics(metrics_summary.all_diffs());

        let expected = format!(
            "  {:<21}{new:>METRIC_WIDTH$}|{:<METRIC_WIDTH$} ({diff_pct}){}\n",
            format!("{event_kind}:"),
            old.map_or(NOT_AVAILABLE.to_owned(), |o| o.to_string()),
            diff_fact.map_or_else(String::new, |f| format!(" [{f}]"))
        );

        assert_eq!(formatter.buffer, expected);
    }

    #[rstest]
    #[case::normal_no_grid(IndentKind::Normal, false, "  ")]
    #[case::tool_header_no_grid(IndentKind::ToolHeadline, false, "  ")]
    #[case::tool_sub_header_no_grid(IndentKind::ToolSubHeadline, false, "  ")]
    #[case::normal_with_grid(IndentKind::Normal, true, "| ")]
    #[case::tool_header_with_grid(IndentKind::ToolHeadline, true, "|=")]
    #[case::tool_sub_header_with_grid(IndentKind::ToolSubHeadline, true, "|-")]
    fn test_vertical_formatter_write_indent(
        #[case] kind: IndentKind,
        #[case] show_grid: bool,
        #[case] expected: &str,
    ) {
        colored::control::set_override(false);

        let output_format = OutputFormat {
            show_grid,
            ..Default::default()
        };

        let mut formatter = VerticalFormatter::new(output_format);
        formatter.write_indent(&kind);
        assert_eq!(formatter.buffer, expected);
    }

    #[rstest]
    #[case::left(
        "Some:",
        EitherOrBoth::Left("left"),
        "  Some:                                left\n"
    )]
    #[case::right(
        "Field:",
        EitherOrBoth::Right("right"),
        "  Field:                                   |right\n"
    )]
    #[case::both(
        "Field:",
        EitherOrBoth::Both("left", "right"),
        "  Field:                               left|right\n"
    )]
    #[case::both_u64_max(
        "Field:",
        EitherOrBoth::Both(format!("{}", u64::MAX), format!("{}", u64::MAX)),
        "  Field:               18446744073709551615|18446744073709551615\n"
    )]
    #[case::split(
        "Field:",
        EitherOrBoth::Both(format!("{}1", u64::MAX), "right".to_owned()),
        "  Field:               184467440737095516151\n                                           |right\n"
    )]
    fn test_vertical_formatter_write_field<T>(
        #[case] field: &str,
        #[case] values: EitherOrBoth<T>,
        #[case] expected: &str,
    ) where
        T: AsRef<str>,
    {
        colored::control::set_override(false);

        let output_format = OutputFormat::default();

        let mut formatter = VerticalFormatter::new(output_format);
        formatter.write_field(field, &values, None, false);
        assert_eq!(formatter.buffer, expected);
    }
}
