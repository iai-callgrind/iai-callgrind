use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;

use super::ToolOutputPath;
use crate::error::Error;
use crate::runner::summary::{CostsSummary, ErrorSummary};
use crate::runner::tool::Parser;
use crate::util::make_relative;

// The different regex have to consider --time-stamp=yes
lazy_static! {
    static ref EXTRACT_FIELDS_RE: Regex = regex::Regex::new(
        r"^\s*(==|--)([0-9:.]+\s+)?[0-9]+(==|--)\s*(?<key>.*?)\s*:\s*(?<value>.*)\s*$"
    )
    .expect("Regex should compile");
    static ref EMPTY_LINE_RE: Regex =
        regex::Regex::new(r"^\s*(==|--)([0-9:.]+\s+)?[0-9]+(==|--)\s*$")
            .expect("Regex should compile");
    static ref STRIP_PREFIX_RE: Regex =
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

pub struct LogfileParser {
    pub root_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct LogfileSummary {
    pub command: PathBuf,
    pub pid: i32,
    pub parent_pid: Option<i32>,
    pub fields: Vec<(String, String)>,
    pub details: Vec<String>,
    pub error_summary: Option<ErrorSummary>,
    pub cost_summary: Option<CostsSummary<String>>,
    pub log_path: PathBuf,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    Header,
    HeaderSpace,
    Body,
}

impl LogfileParser {
    fn parse_single(&self, path: PathBuf) -> Result<LogfileSummary> {
        let file = File::open(&path)
            .with_context(|| format!("Error opening log file '{}'", path.display()))?;

        let mut iter = BufReader::new(file)
            .lines()
            .map(std::result::Result::unwrap)
            .skip_while(|l| l.trim().is_empty());

        let line = iter
            .next()
            .ok_or_else(|| Error::ParseError((path.clone(), "Empty file".to_owned())))?;
        let pid = EXTRACT_PID_RE
            .captures(line.trim())
            .expect("Log output should not be malformed")
            .name("pid")
            .expect("Log output should contain pid")
            .as_str()
            .parse::<i32>()
            .expect("Pid should be valid");

        let mut state = State::Header;
        let mut command = None;
        let mut details = vec![];
        let mut error_summary = None;
        let mut parent_pid = None;
        for line in iter {
            match &state {
                State::Header if !EMPTY_LINE_RE.is_match(&line) => {
                    if let Some(caps) = EXTRACT_FIELDS_RE.captures(&line) {
                        let key = caps.name("key").unwrap().as_str();
                        match key.to_ascii_lowercase().as_str() {
                            "command" => {
                                let value = caps.name("value").unwrap().as_str();
                                command = Some(make_relative(&self.root_dir, value));
                            }
                            "parent pid" => {
                                let value = caps.name("value").unwrap().as_str().to_owned();
                                parent_pid = Some(
                                    value
                                        .as_str()
                                        .parse::<i32>()
                                        .expect("Parent PID should be valid"),
                                );
                            }
                            _ => {}
                        }
                    }
                }
                State::Header => state = State::HeaderSpace,
                State::HeaderSpace if EMPTY_LINE_RE.is_match(&line) => {}
                State::HeaderSpace | State::Body => {
                    if state == State::HeaderSpace {
                        state = State::Body;
                    }
                    if let Some(caps) = EXTRACT_FIELDS_RE.captures(&line) {
                        let key = caps.name("key").unwrap().as_str();
                        if key.eq_ignore_ascii_case("error summary") {
                            let error_summary_value = caps.name("value").unwrap().as_str();
                            error_summary =
                                ErrorSummary::from_str(error_summary_value).map(Some)?;

                            continue;
                        }
                    }
                    if let Some(caps) = STRIP_PREFIX_RE.captures(&line) {
                        let rest_of_line = caps.name("rest").unwrap().as_str();
                        details.push(rest_of_line.to_owned());
                    } else {
                        details.push(line);
                    }
                }
            }
        }

        while let Some(last) = details.last() {
            if last.trim().is_empty() {
                details.pop();
            } else {
                break;
            }
        }

        Ok(LogfileSummary {
            command: command.expect("A command should be present"),
            pid,
            parent_pid,
            fields: Vec::default(),
            details,
            error_summary,
            log_path: make_relative(&self.root_dir, path),
            cost_summary: None,
        })
    }
}

impl Parser for LogfileParser {
    type Output = Vec<LogfileSummary>;

    fn parse(&self, output_path: &ToolOutputPath) -> Result<Self::Output>
    where
        Self: std::marker::Sized,
    {
        let log_path = output_path.to_log_output();
        debug!("{}: Parsing log file '{}'", output_path.tool.id(), log_path);

        let mut summaries = vec![];
        for path in log_path.real_paths()? {
            let summary = self.parse_single(path)?;
            summaries.push(summary);
        }
        summaries.sort_by_key(|s| s.pid);
        Ok(summaries)
    }
}

impl LogfileSummary {
    pub fn has_errors(&self) -> bool {
        self.error_summary
            .as_ref()
            .map_or(false, ErrorSummary::has_errors)
    }
}
