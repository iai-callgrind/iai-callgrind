use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use itertools::Itertools;
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;

use super::error_metric_parser::ErrorMetricLogfileParser;
use super::generic_parser::GenericLogfileParser;
use super::{ToolOutputPath, ValgrindTool};
use crate::error::Error;
use crate::runner::cachegrind::summary_parser::SummaryParser;
use crate::runner::dhat::logfile_parser::DhatLogfileParser;
use crate::runner::summary::{
    SegmentDetails, ToolMetricSummary, ToolMetrics, ToolRun, ToolRunSegment,
};
use crate::util::EitherOrBoth;

// The different regex have to consider --time-stamp=yes
lazy_static! {
    pub static ref EXTRACT_FIELDS_RE: Regex = regex::Regex::new(
        r"^\s*(==|--)([0-9:.]+\s+)?[0-9]+(==|--)\s*(?<key>.*?)\s*:\s*(?<value>.*)\s*$"
    )
    .expect("Regex should compile");
    pub static ref EMPTY_LINE_RE: Regex =
        regex::Regex::new(r"^\s*(==|--)([0-9:.]+\s+)?[0-9]+(==|--)\s*$")
            .expect("Regex should compile");
    pub static ref STRIP_PREFIX_RE: Regex =
        regex::Regex::new(r"^\s*(==|--)([0-9:.]+\s+)?[0-9]+(==|--) (?<rest>.*)$")
            .expect("Regex should compile");
    static ref EXTRACT_PID_RE: Regex =
        regex::Regex::new(r"^\s*(==|--)([0-9:.]+\s+)?(?<pid>[0-9]+)(==|--).*")
            .expect("Regex should compile");
}

// TODO: Adjust variables, messages to rename and move into parser module?
// TODO: Adjust usages names like logfile_parser etc.
pub trait Parser {
    fn parse_single(&self, path: PathBuf) -> Result<ParserResult>;
    fn parse_with(&self, output_path: &ToolOutputPath) -> Result<Vec<ParserResult>> {
        debug!("{}: Parsing file '{}'", output_path.tool.id(), output_path);

        let mut parser_results = vec![];
        let Ok(paths) = output_path.real_paths() else {
            return Ok(vec![]);
        };

        for path in paths {
            let logfile = self.parse_single(path)?;
            parser_results.push(logfile);
        }

        parser_results.sort_by_key(|x| x.header.pid);
        Ok(parser_results)
    }

    fn parse(&self) -> Result<Vec<ParserResult>> {
        self.parse_with(self.get_output_path())
    }

    fn parse_base(&self) -> Result<Vec<ParserResult>> {
        self.parse_with(&self.get_output_path().to_base_path())
    }

    fn get_output_path(&self) -> &ToolOutputPath;
}

// TODO: Header should include the callgrind stuff too
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub command: String,
    pub pid: i32,
    pub parent_pid: Option<i32>,
    pub desc: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParserResult {
    pub path: PathBuf,
    pub header: Header,
    pub details: Vec<String>,
    pub metrics: ToolMetrics,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LogfileSummary {
    pub logfile: EitherOrBoth<ParserResult>,
    pub metrics_summary: ToolMetricSummary,
}

impl From<ParserResult> for SegmentDetails {
    fn from(value: ParserResult) -> Self {
        Self {
            command: value.header.command,
            pid: value.header.pid,
            parent_pid: value.header.parent_pid,
            details: (!value.details.is_empty()).then(|| value.details.join("\n")),
            path: value.path,
            part: None,
            thread: None,
        }
    }
}

// Logfiles are separated per process but not per threads by any tool
impl From<EitherOrBoth<Vec<ParserResult>>> for ToolRun {
    fn from(logfiles: EitherOrBoth<Vec<ParserResult>>) -> Self {
        let mut total: Option<ToolMetricSummary> = None;

        let segments: Vec<ToolRunSegment> = match logfiles {
            EitherOrBoth::Left(new) => new
                .into_iter()
                .map(|logfile| {
                    let metrics_summary = ToolMetricSummary::from_new_metrics(&logfile.metrics);
                    if let Some(entry) = total.as_mut() {
                        entry.add_mut(&metrics_summary);
                    } else {
                        total = Some(metrics_summary.clone());
                    }

                    ToolRunSegment {
                        details: EitherOrBoth::Left(logfile.into()),
                        metrics_summary,
                    }
                })
                .collect(),
            EitherOrBoth::Right(old) => old
                .into_iter()
                .map(|logfile| {
                    let metrics_summary = ToolMetricSummary::from_old_metrics(&logfile.metrics);
                    if let Some(entry) = total.as_mut() {
                        entry.add_mut(&metrics_summary);
                    } else {
                        total = Some(metrics_summary.clone());
                    }

                    ToolRunSegment {
                        details: EitherOrBoth::Right(logfile.into()),
                        metrics_summary,
                    }
                })
                .collect(),
            EitherOrBoth::Both(new, old) => new
                .into_iter()
                .zip_longest(old)
                .map(|either_or_both| match either_or_both {
                    itertools::EitherOrBoth::Both(new, old) => {
                        let metrics_summary = ToolMetricSummary::try_from_new_and_old_metrics(
                            &new.metrics,
                            &old.metrics,
                        )
                        .expect("The cost kinds should match");

                        if let Some(entry) = total.as_mut() {
                            entry.add_mut(&metrics_summary);
                        } else {
                            total = Some(metrics_summary.clone());
                        }

                        ToolRunSegment {
                            details: EitherOrBoth::Both(new.into(), old.into()),
                            metrics_summary,
                        }
                    }
                    itertools::EitherOrBoth::Left(new) => {
                        let metrics_summary = ToolMetricSummary::from_new_metrics(&new.metrics);
                        if let Some(entry) = total.as_mut() {
                            entry.add_mut(&metrics_summary);
                        } else {
                            total = Some(metrics_summary.clone());
                        }

                        ToolRunSegment {
                            details: EitherOrBoth::Left(new.into()),
                            metrics_summary,
                        }
                    }
                    itertools::EitherOrBoth::Right(old) => {
                        let metrics_summary = ToolMetricSummary::from_old_metrics(&old.metrics);
                        if let Some(entry) = total.as_mut() {
                            entry.add_mut(&metrics_summary);
                        } else {
                            total = Some(metrics_summary.clone());
                        }

                        ToolRunSegment {
                            details: EitherOrBoth::Right(old.into()),
                            metrics_summary,
                        }
                    }
                })
                .collect(),
        };

        Self {
            segments,
            total: total.expect("A total should be present"),
        }
    }
}

pub fn extract_pid(line: &str) -> Result<i32> {
    EXTRACT_PID_RE
        .captures(line.trim())
        .context("Log output should not be malformed")?
        .name("pid")
        .context("Log output should contain pid")?
        .as_str()
        .parse::<i32>()
        .context("Pid should be valid")
}

/// Parse the logfile header
///
/// The logfile header is the same for all tools
pub fn parse_header(path: &Path, mut lines: impl Iterator<Item = String>) -> Result<Header> {
    let next = lines.next();

    let (pid, next) = if let Some(next) = next {
        (extract_pid(&next)?, next)
    } else {
        return Err(Error::ParseError((path.to_owned(), "Empty file".to_owned())).into());
    };

    let mut parent_pid = None;
    let mut command = None;
    for line in std::iter::once(next).chain(lines) {
        if EMPTY_LINE_RE.is_match(&line) {
            // The header is separated from the body by at least one empty line. The first
            // empty line is removed from the iterator.
            break;
        } else if let Some(caps) = EXTRACT_FIELDS_RE.captures(&line) {
            let key = caps.name("key").unwrap().as_str();

            // These unwraps are safe. If there is a key, there is also a value present
            match key.to_ascii_lowercase().as_str() {
                "command" => {
                    let value = caps.name("value").unwrap().as_str();
                    command = Some(value.to_owned());
                }
                "parent pid" => {
                    let value = caps.name("value").unwrap().as_str().to_owned();
                    parent_pid = Some(
                        value
                            .as_str()
                            .parse::<i32>()
                            .context("Failed parsing log file: Parent pid should be valid")?,
                    );
                }
                _ => {
                    // Ignore other header lines
                }
            }
        } else {
            // Some malformed header line which we ignore
        }
    }

    Ok(Header {
        command: command.with_context(|| {
            format!(
                "Error parsing header of logfile '{}': A command should be present",
                path.display()
            )
        })?,
        pid,
        parent_pid,
        desc: vec![],
    })
}

pub fn parser_factory(
    tool: ValgrindTool,
    root_dir: PathBuf,
    output_path: &ToolOutputPath,
) -> Box<dyn Parser> {
    match tool {
        ValgrindTool::Cachegrind => Box::new(SummaryParser {
            output_path: output_path.clone(),
        }),
        ValgrindTool::DHAT => Box::new(DhatLogfileParser {
            output_path: output_path.to_log_output(),
            root_dir,
        }),
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
    #[case::equals_sign(
        "==1746070== Cachegrind, a high-precision tracing profiler",
        1_746_070_i32
    )]
    #[case::hyphen(
        "--1746070-- warning: L3 cache found, using its data for the LL simulation.",
        1_746_070_i32
    )]
    #[case::timestamp(
        "==00:00:00:00.000 1811497== Callgrind, a call-graph generating cache profiler",
        1_811_497_i32
    )]
    fn test_extract_pid(#[case] haystack: &str, #[case] expected: i32) {
        let actual = extract_pid(haystack).unwrap();
        assert_eq!(actual, expected);
    }
}
