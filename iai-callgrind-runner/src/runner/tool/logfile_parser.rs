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
use crate::runner::dhat::logfile_parser::DhatLogfileParser;
use crate::runner::summary::{
    SegmentDetails, ToolMetricSummary, ToolMetrics, ToolRun, ToolRunSegment,
};
use crate::util::{make_relative, EitherOrBoth};

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
    static ref EXTRACT_ERRORS_RE: Regex =
        regex::Regex::new(r"^.*?(?<errors>[0-9]+).*$").expect("Regex should compile");
    static ref EXTRACT_ERROR_SUMMARY_RE: Regex = regex::Regex::new(
        r"^.*?(?<err>[0-9]+).*(<?<ctxs>[0-9]+).*(<?<s_err>[0-9]+).*(<?<s_ctxs>[0-9]+)$"
    )
    .expect("Regex should compile");
}

pub trait LogfileParser {
    fn parse_single(&self, path: PathBuf) -> Result<Logfile>;
    fn parse(&self, output_path: &ToolOutputPath) -> Result<Vec<Logfile>> {
        let log_path = output_path.to_log_output();
        debug!("{}: Parsing log file '{}'", output_path.tool.id(), log_path);

        let mut logfiles = vec![];
        let Ok(paths) = log_path.real_paths() else {
            return Ok(vec![]);
        };

        for path in paths {
            let logfile = self.parse_single(path)?;
            logfiles.push(logfile);
        }

        logfiles.sort_by_key(|x| x.header.pid);
        Ok(logfiles)
    }
}

#[derive(Debug)]
pub struct Header {
    pub command: PathBuf,
    pub pid: i32,
    pub parent_pid: Option<i32>,
}

#[derive(Debug)]
pub struct Logfile {
    pub path: PathBuf,
    pub header: Header,
    pub details: Vec<String>,
    pub metrics: ToolMetrics,
}

#[derive(Debug)]
pub struct LogfileSummary {
    pub logfile: EitherOrBoth<Logfile>,
    pub metrics_summary: ToolMetricSummary,
}

impl From<Logfile> for SegmentDetails {
    fn from(value: Logfile) -> Self {
        Self {
            command: value.header.command.display().to_string(),
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
impl From<EitherOrBoth<Vec<Logfile>>> for ToolRun {
    fn from(logfiles: EitherOrBoth<Vec<Logfile>>) -> Self {
        let mut total: Option<ToolMetricSummary> = None;

        let segments: Vec<ToolRunSegment> = match logfiles {
            EitherOrBoth::Left(new) => new
                .into_iter()
                .map(|logfile| {
                    let metrics_summary = ToolMetricSummary::from_new_costs(&logfile.metrics);
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
                    let metrics_summary = ToolMetricSummary::from_old_costs(&logfile.metrics);
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
                        let metrics_summary = ToolMetricSummary::try_from_new_and_old_costs(
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
                        let metrics_summary = ToolMetricSummary::from_new_costs(&new.metrics);
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
                        let metrics_summary = ToolMetricSummary::from_old_costs(&old.metrics);
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

pub fn extract_pid(line: &str) -> i32 {
    // TODO: Return error instead of unwraps
    EXTRACT_PID_RE
        .captures(line.trim())
        .expect("Log output should not be malformed")
        .name("pid")
        .expect("Log output should contain pid")
        .as_str()
        .parse::<i32>()
        .expect("Pid should be valid")
}

pub fn parse_header(
    project_root: &Path,
    path: &Path,
    mut lines: impl Iterator<Item = String>,
) -> Result<Header> {
    let next = lines.next();

    let (pid, next) = if let Some(next) = next {
        (extract_pid(&next), next)
    } else {
        return Err(Error::ParseError((path.to_owned(), "Empty file".to_owned())).into());
    };

    let mut parent_pid = None;
    let mut command = None;
    for line in std::iter::once(next).chain(lines) {
        if EMPTY_LINE_RE.is_match(&line) {
            // The header is separated from the body by at least one empty line. The first
            // empty line is stripped from the iterator.
            break;
        } else if let Some(caps) = EXTRACT_FIELDS_RE.captures(&line) {
            let key = caps.name("key").unwrap().as_str();

            // These unwraps are safe. If there is a key, there is also a value present
            match key.to_ascii_lowercase().as_str() {
                "command" => {
                    let value = caps.name("value").unwrap().as_str();
                    command = Some(make_relative(project_root, value));
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
    })
}

pub fn parser_factory(tool: ValgrindTool, root_dir: PathBuf) -> Box<dyn LogfileParser> {
    match tool {
        ValgrindTool::DHAT => Box::new(DhatLogfileParser { root_dir }),
        ValgrindTool::Memcheck | ValgrindTool::DRD | ValgrindTool::Helgrind => {
            Box::new(ErrorMetricLogfileParser { root_dir })
        }
        _ => Box::new(GenericLogfileParser { root_dir }),
    }
}
