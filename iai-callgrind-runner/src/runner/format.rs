use std::borrow::Cow;
use std::fmt::{Display, Write};
use std::path::PathBuf;

use anyhow::Result;
use colored::{ColoredString, Colorize};

use super::args::NoCapture;
use super::bin_bench::BinBench;
use super::callgrind::parser::CallgrindProperties;
use super::callgrind::Summaries;
use super::common::ModulePath;
use super::lib_bench::LibBench;
use super::meta::Metadata;
use super::summary::{CostsDiff, CostsSummary};
use super::tool::ValgrindTool;
use crate::api::{self, EventKind};
use crate::util::{make_relative, to_string_signed_short, truncate_str_utf8, EitherOrBoth};

// TODO: Increase the possible length of the keys in the vertical output. Increase the space for
// numbers a little bit? Increase the precision of the percentage and factor to 7 significant
// numbers.

pub const NOT_AVAILABLE: &str = "N/A";

pub struct ComparisonHeader {
    pub function_name: String,
    pub id: String,
    pub details: Option<String>,
}

pub struct BinaryBenchmarkHeader {
    inner: Header,
    has_tools_enabled: bool,
    output_format: OutputFormat,
}

struct Header {
    module_path: String,
    id: Option<String>,
    description: Option<String>,
}

pub trait Formatter {
    fn format_float(float: f64, unit: &str) -> ColoredString {
        let signed_short = to_string_signed_short(float);
        if float.is_infinite() {
            if float.is_sign_positive() {
                format!("{signed_short:+^9}").bright_red().bold()
            } else {
                format!("{signed_short:-^9}").bright_green().bold()
            }
        } else if float.is_sign_positive() {
            format!("{signed_short:^+8}{unit}").bright_red().bold()
        } else {
            format!("{signed_short:^+8}{unit}").bright_green().bold()
        }
    }

    fn format(
        &self,
        baselines: (Option<String>, Option<String>),
        costs_summary: &CostsSummary,
    ) -> Result<String>;
}

pub struct LibraryBenchmarkHeader {
    inner: Header,
    has_tools_enabled: bool,
    output_format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Default,
    Json,
    PrettyJson,
}

#[derive(Clone)]
pub struct VerticalFormat {
    event_kinds: Vec<EventKind>,
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
                bin_bench.truncate_description,
            ),
            has_tools_enabled: bin_bench.tools.has_tools_enabled(),
            output_format: meta.args.output_format,
        }
    }

    pub fn print(&self) {
        if self.output_format == OutputFormat::Default {
            self.inner.print();
            if self.has_tools_enabled {
                println!("{}", tool_headline(ValgrindTool::Callgrind));
            }
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
    pub fn new<T, U, V>(function_name: T, id: U, details: Option<V>) -> Self
    where
        T: Into<String>,
        U: Into<String>,
        V: Into<String>,
    {
        Self {
            function_name: function_name.into(),
            id: id.into(),
            details: details.map(Into::into),
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
            "  {} {} {}",
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
        truncate_description: Option<usize>,
    ) -> Self
    where
        T: Into<Option<String>>,
    {
        let truncated =
            description.map(|d| self::truncate_description(&d, truncate_description).to_string());

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
        write!(&mut output, "{}", self.module_path).unwrap();
        if let Some(id) = &self.id {
            match &self.description {
                Some(description) if !description.is_empty() => {
                    write!(&mut output, " {id}:{description}").unwrap();
                }
                _ => {
                    write!(&mut output, " {id}").unwrap();
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
    pub fn new(meta: &Metadata, lib_bench: &LibBench) -> Self {
        let header = Header::new(
            &lib_bench.module_path,
            lib_bench.id.clone(),
            lib_bench.args.clone(),
            lib_bench.truncate_description,
        );

        Self {
            inner: header,
            has_tools_enabled: lib_bench.tools.has_tools_enabled(),
            output_format: meta.args.output_format,
        }
    }

    pub fn print(&self) {
        if self.output_format == OutputFormat::Default {
            self.inner.print();
            if self.has_tools_enabled {
                println!("{}", tool_headline(ValgrindTool::Callgrind));
            }
        }
    }

    pub fn to_title(&self) -> String {
        self.inner.to_title()
    }

    pub fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }
}

impl VerticalFormat {
    // TODO: CLEANUP?
    pub fn print(
        &self,
        meta: &Metadata,
        baselines: (Option<String>, Option<String>),
        costs_summary: &CostsSummary,
    ) -> Result<()> {
        if meta.args.output_format == OutputFormat::Default {
            print!("{}", self.format(baselines, costs_summary)?);
        }
        Ok(())
    }

    pub fn print_multiple(
        &self,
        meta: &Metadata,
        baselines: (Option<String>, Option<String>),
        summaries: &Summaries,
    ) -> Result<()> {
        if meta.args.output_format == OutputFormat::Default {
            if summaries.has_multiple() {
                for summary in &summaries.data {
                    println!("{}", callgrind_multiple_files_header(&summary.details));
                    print!(
                        "{}",
                        self.format(baselines.clone(), &summary.costs_summary)?
                    );
                }
                println!("{}", callgrind_total_header());
            }
            print!("{}", self.format(baselines, &summaries.total)?);
        }
        Ok(())
    }
}

impl Default for VerticalFormat {
    fn default() -> Self {
        use EventKind::*;
        Self {
            event_kinds: vec![
                Ir,
                L1hits,
                LLhits,
                RamHits,
                TotalRW,
                EstimatedCycles,
                SysCount,
                SysTime,
                SysCpuTime,
                Ge,
                Bc,
                Bcm,
                Bi,
                Bim,
                ILdmr,
                DLdmr,
                DLdmw,
                AcCost1,
                AcCost2,
                SpLoss1,
                SpLoss2,
            ],
        }
    }
}

impl Formatter for VerticalFormat {
    fn format(
        &self,
        baselines: (Option<String>, Option<String>),
        costs_summary: &CostsSummary,
    ) -> Result<String> {
        format_vertical(
            baselines,
            self.event_kinds
                .iter()
                .filter_map(|e| costs_summary.diff_by_kind(e).map(|d| (e, d))),
        )
    }
}

pub fn format_vertical<'a, K: Display + 'a>(
    baselines: (Option<String>, Option<String>),
    costs_summary: impl Iterator<Item = (&'a K, &'a CostsDiff)>,
) -> Result<String> {
    let mut result = String::new();

    let unknown = "*********";
    let no_change = "No change";

    // Move this into a function format_baselines_header
    match baselines {
        (None, None) => {}
        (None, Some(base)) => {
            writeln!(result, "  {:<33}|{base}", "Baselines:").unwrap();
        }
        (Some(base), None) => {
            writeln!(result, "  {:<18}{:>15}", "Baselines:", base.bold()).unwrap();
        }
        (Some(new), Some(old)) => {
            writeln!(result, "  {:<18}{:>15}|{old}", "Baselines:", new.bold()).unwrap();
        }
    }

    for (event_kind, diff) in costs_summary {
        let description = format!("{event_kind}:");
        match diff.costs {
            EitherOrBoth::Left(new_cost) => writeln!(
                result,
                "  {description:<18}{:>15}|{NOT_AVAILABLE:<15} ({:^9})",
                new_cost.to_string().bold(),
                unknown.bright_black()
            )?,
            EitherOrBoth::Right(old_cost) => writeln!(
                result,
                "  {description:<18}{:>15}|{old_cost:<15} ({:^9})",
                NOT_AVAILABLE.bold(),
                unknown.bright_black()
            )?,
            EitherOrBoth::Both(new_cost, old_cost) if new_cost == old_cost => writeln!(
                result,
                "  {description:<18}{:>15}|{old_cost:<15} ({:^9})",
                new_cost.to_string().bold(),
                no_change.bright_black()
            )?,
            EitherOrBoth::Both(new_cost, old_cost) => {
                let diffs = diff.diffs.expect(
                    "If there are new costs and old costs there should be a difference present",
                );
                let pct_string = VerticalFormat::format_float(diffs.diff_pct, "%");
                let factor_string = VerticalFormat::format_float(diffs.factor, "x");
                writeln!(
                    result,
                    "  {description:<18}{:>15}|{old_cost:<15} ({pct_string:^9}) \
                     [{factor_string:^9}]",
                    new_cost.to_string().bold(),
                )?;
            }
        }
    }
    Ok(result)
}

pub fn callgrind_multiple_files_header(
    properties: &EitherOrBoth<(PathBuf, CallgrindProperties)>,
) -> String {
    fn fields(property: &CallgrindProperties) -> String {
        let pid = property
            .pid
            .map_or(NOT_AVAILABLE.to_owned(), |v| v.to_string());
        let part = property
            .part
            .map_or(NOT_AVAILABLE.to_owned(), |v| v.to_string());
        let thread = property
            .thread
            .map_or(NOT_AVAILABLE.to_owned(), |v| v.to_string());
        format!("pid: {pid} part: {part} thread: {thread}")
    }

    let max_left = 31;
    let hash = "##".yellow();
    match properties {
        EitherOrBoth::Left(new) => {
            let left = fields(&new.1);
            let len = left.len();
            let left = left.bold();
            if len > max_left {
                format!(
                    "  {hash} {left}\n{}|{NOT_AVAILABLE}",
                    " ".repeat(max_left + 4).yellow()
                )
            } else {
                format!(
                    "  {hash} {left}{}|{NOT_AVAILABLE}",
                    " ".repeat(max_left - len - 1)
                )
            }
        }
        EitherOrBoth::Right(old) => {
            let right = fields(&old.1);
            format!(
                "  {hash} {}{}|{right}",
                NOT_AVAILABLE.bold(),
                " ".repeat(max_left - NOT_AVAILABLE.len() - 1)
            )
        }
        EitherOrBoth::Both(new, old) => {
            let left = fields(&new.1);
            let len = left.len();
            let right = fields(&old.1);
            let left = left.bold();
            if len > max_left {
                format!(
                    "  {hash} {left}\n{}|{right}",
                    " ".repeat(max_left + 4).yellow()
                )
            } else {
                format!("  {hash} {left}{}|{right}", " ".repeat(max_left - len - 1))
            }
        }
    }
}

pub fn callgrind_total_header() -> String {
    format!("  {} Total", "##".yellow())
}

pub fn tool_headline(tool: ValgrindTool) -> String {
    let id = tool.id();
    format!(
        "  {} {} {}",
        "=======".bright_black(),
        id.to_ascii_uppercase(),
        "=".repeat(64 - id.len()).bright_black(),
        // "=".repeat(34 - tool.id().len()).bright_black()
    )
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
    use crate::runner::costs::Costs;

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

        let header = Header::new(
            &ModulePath::new(module_path),
            id.map(ToOwned::to_owned),
            description.map(ToOwned::to_owned),
            None,
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

        let header = Header::new(
            &ModulePath::new(module_path),
            id.map(ToOwned::to_owned),
            description.map(ToOwned::to_owned),
            truncate_description,
        );

        assert_eq!(header.to_string(), expected);
    }

    // TODO: Add more tests for the format. This tests only very basically only a single line and if
    // new costs are present.
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
        colored::control::set_override(false);

        let costs = match old {
            Some(old) => EitherOrBoth::Both(
                Costs(indexmap! {event_kind => new}),
                Costs(indexmap! {event_kind => old}),
            ),
            None => EitherOrBoth::Left(Costs(indexmap! {event_kind => new})),
        };
        let costs_summary = CostsSummary::new(costs);
        let formatted = format_vertical((None, None), costs_summary.all_diffs()).unwrap();

        let expected = format!(
            "  {:<18}{new:>15}|{:<15} ({diff_pct}){}\n",
            format!("{event_kind}:"),
            old.map_or(NOT_AVAILABLE.to_owned(), |o| o.to_string()),
            diff_fact.map_or_else(String::new, |f| format!(" [{f}]"))
        );

        assert_eq!(formatted, expected);
    }
}
