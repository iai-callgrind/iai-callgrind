use std::borrow::Cow;
use std::fmt::{Display, Write};

use anyhow::Result;
use colored::{ColoredString, Colorize};

use super::args::NoCapture;
use super::meta::Metadata;
use super::summary::{CostsDiff, CostsSummary};
use super::tool::ValgrindTool;
use crate::api::EventKind;
use crate::util::{to_string_signed_short, truncate_str_utf8};

pub const NOT_AVAILABLE: &str = "N/A";

pub struct ComparisonHeader {
    pub function_name: String,
    pub id: String,
    pub details: Option<String>,
}

pub struct Header {
    pub module_path: String,
    pub id: Option<String>,
    pub description: Option<String>,
    pub truncate_description: Option<usize>,
}

pub trait Formatter {
    fn format_float(float: f64, unit: &str) -> ColoredString {
        let signed_short = to_string_signed_short(float);
        if float.is_infinite() {
            if float.is_sign_positive() {
                format!("{signed_short:+^9}").bright_red().bold()
            } else {
                format!("{signed_short:+^9}").bright_green().bold()
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
    pub fn new<T, U, V>(
        module_path: T,
        id: U,
        description: V,
        truncate_description: Option<usize>,
    ) -> Self
    where
        T: Into<String>,
        U: Into<Option<String>>,
        V: Into<Option<String>>,
    {
        Self {
            module_path: module_path.into(),
            id: id.into(),
            description: description.into(),
            truncate_description,
        }
    }

    pub fn from_segments<I, T, U, V>(
        module_path: T,
        id: U,
        description: V,
        truncate_description: Option<usize>,
    ) -> Self
    where
        I: AsRef<str>,
        T: AsRef<[I]>,
        U: Into<Option<String>>,
        V: Into<Option<String>>,
    {
        Self {
            module_path: module_path
                .as_ref()
                .iter()
                .map(|s| s.as_ref().to_owned())
                .collect::<Vec<String>>()
                .join("::"),
            id: id.into(),
            description: description.into(),
            truncate_description,
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
                    let truncated = self.truncate_description(description);
                    write!(&mut output, " {id}:{truncated}").unwrap();
                }
                _ => {
                    write!(&mut output, " {id}").unwrap();
                }
            }
        }
        output
    }

    pub fn truncate_description<'a>(&self, description: &'a str) -> Cow<'a, str> {
        if let Some(num) = self.truncate_description {
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
}

impl Display for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.module_path.green()))?;
        if let Some(id) = &self.id {
            match &self.description {
                Some(description) if !description.is_empty() => {
                    let truncated = self.truncate_description(description);
                    f.write_fmt(format_args!(
                        " {}{}{}",
                        id.cyan(),
                        ":".cyan(),
                        truncated.bold().blue(),
                    ))?;
                }
                _ => {
                    f.write_fmt(format_args!(" {}", id.cyan()))?;
                }
            }
        }
        Ok(())
    }
}

impl VerticalFormat {
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
        match (diff.new, diff.old) {
            (None, Some(old_cost)) => writeln!(
                result,
                "  {description:<18}{:>15}|{old_cost:<15} ({:^9})",
                NOT_AVAILABLE.bold(),
                unknown.bright_black()
            )?,
            (Some(new_cost), None) => writeln!(
                result,
                "  {description:<18}{:>15}|{NOT_AVAILABLE:<15} ({:^9})",
                new_cost.to_string().bold(),
                unknown.bright_black()
            )?,
            (Some(new_cost), Some(old_cost)) if new_cost == old_cost => writeln!(
                result,
                "  {description:<18}{:>15}|{old_cost:<15} ({:^9})",
                new_cost.to_string().bold(),
                no_change.bright_black()
            )?,
            (Some(new_cost), Some(old_cost)) => {
                let pct_string = {
                    let pct = diff.diff_pct.expect(
                        "If there are new costs and old costs there should be a difference in \
                         percent",
                    );
                    VerticalFormat::format_float(pct, "%")
                };
                let factor_string = {
                    let factor = diff.factor.expect(
                        "If there are new costs and old costs there should be a difference factor",
                    );
                    VerticalFormat::format_float(factor, "x")
                };
                writeln!(
                    result,
                    "  {description:<18}{:>15}|{old_cost:<15} ({pct_string:^9}) \
                     [{factor_string:^9}]",
                    new_cost.to_string().bold(),
                )?;
            }
            _ => {}
        }
    }
    Ok(result)
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

pub fn no_capture_footer(nocapture: NoCapture) -> String {
    match nocapture {
        NoCapture::True => format!("{} {}", "-".yellow(), "end of stdout/stderr".yellow()),
        NoCapture::False => String::new(),
        NoCapture::Stderr => format!("{} {}", "-".yellow(), "end of stderr".yellow()),
        NoCapture::Stdout => format!("{} {}", "-".yellow(), "end of stdout".yellow()),
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::simple("some::module", Some("id"), Some("1, 2"), "some::module id:1, 2")]
    #[case::no_id_but_description("some::module", None, Some("1, 2, 3"), "some::module")]
    #[case::id_no_description("some::module", Some("id"), None, "some::module id")]
    #[case::description_is_empty("some::module", Some("id"), Some(""), "some::module id")]
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
            module_path,
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
            module_path,
            id.map(ToOwned::to_owned),
            description.map(ToOwned::to_owned),
            truncate_description,
        );

        assert_eq!(header.to_string(), expected);
    }
}
