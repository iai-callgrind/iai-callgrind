use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use lazy_static::lazy_static;
use regex::Regex;

use super::logfile_parser::{
    extract_pid, LogfileParser, LogfileSummary, EMPTY_LINE_RE, EXTRACT_FIELDS_RE, STRIP_PREFIX_RE,
};
use crate::api::ErrorMetricKind;
use crate::error::Error;
use crate::runner::costs::Costs;
use crate::runner::summary::{CostsKind, ToolRunSummary};
use crate::util::make_relative;

lazy_static! {
    static ref EXTRACT_ERROR_SUMMARY_RE: Regex = regex::Regex::new(
        r"^.*(?<errs>[0-9]+).*(?<ctxs>[0-9]+).*(?<s_errs>[0-9]+).*(?<s_ctxs>[0-9]+).*$"
    )
    .expect("Regex should compile");
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    Header,
    HeaderSpace,
    Body,
}

pub struct ErrorMetricLogfileParser {
    pub root_dir: PathBuf,
}

impl LogfileParser for ErrorMetricLogfileParser {
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

        let pid = extract_pid(&line);

        let mut parent_pid = None;
        let mut command = None;
        let mut details = vec![];
        let mut costs = Costs::from_iter([
            ErrorMetricKind::Errors,
            ErrorMetricKind::Contexts,
            ErrorMetricKind::SuppressedErrors,
            ErrorMetricKind::SuppressedContexts,
        ]);

        let mut state = State::Header;
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

                            let caps = EXTRACT_ERROR_SUMMARY_RE
                                .captures(error_summary_value)
                                .ok_or(anyhow!(
                                    "Failed to extract error summary from string".to_owned()
                                ))?;

                            costs.add_iter_str([
                                caps.name("errs").unwrap().as_str(),
                                caps.name("ctxs").unwrap().as_str(),
                                caps.name("s_errs").unwrap().as_str(),
                                caps.name("s_ctxs").unwrap().as_str(),
                            ]);
                            continue;
                        }
                    }

                    // Detail lines might also be matched with `EXTRACT_FIELDS_RE`
                    if let Some(caps) = STRIP_PREFIX_RE.captures(&line) {
                        let rest_of_line = caps.name("rest").unwrap().as_str();
                        details.push(rest_of_line.to_owned());
                    } else {
                        details.push(line);
                    }
                }
            }
        }

        // Remove the last empty lines from the details
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
            details,
            log_path: make_relative(&self.root_dir, path),
            costs: CostsKind::ErrorCosts(costs),
        })
    }

    fn merge_logfile_summaries(
        &self,
        _: Vec<LogfileSummary>,
        new: Vec<LogfileSummary>,
    ) -> Vec<ToolRunSummary> {
        new.into_iter()
            .map(LogfileSummary::new_into_tool_run)
            .collect()
    }
}
