use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::{Context, Result};

use super::logfile_parser::{
    extract_pid, LogfileParser, LogfileSummary, EMPTY_LINE_RE, EXTRACT_FIELDS_RE, STRIP_PREFIX_RE,
};
use crate::error::Error;
use crate::runner::summary::{CostsKind, ToolRunSummary};
use crate::util::make_relative;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    Header,
    HeaderSpace,
    Body,
}

pub struct GenericLogfileParser {
    pub root_dir: PathBuf,
}

impl LogfileParser for GenericLogfileParser {
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

        let mut state = State::Header;
        for line in iter {
            match &state {
                State::Header if !EMPTY_LINE_RE.is_match(&line) => {
                    if let Some(caps) = EXTRACT_FIELDS_RE.captures(&line) {
                        let key = caps.name("key").unwrap().as_str();

                        // These unwraps are safe. If there is a key, there is also a value present
                        match key.to_ascii_lowercase().as_str() {
                            "command" => {
                                let value = caps.name("value").unwrap().as_str();
                                command = Some(make_relative(&self.root_dir, value));
                            }
                            "parent pid" => {
                                let value = caps.name("value").unwrap().as_str().to_owned();
                                parent_pid = Some(value.as_str().parse::<i32>().context(
                                    "Failed parsing log file: Parent pid should be valid",
                                )?);
                            }
                            _ => {
                                // Ignore other header lines
                            }
                        }
                    }
                }
                State::Header => state = State::HeaderSpace,
                State::HeaderSpace if EMPTY_LINE_RE.is_match(&line) => {}
                State::HeaderSpace | State::Body => {
                    if state == State::HeaderSpace {
                        state = State::Body;
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

        // Remove the last empty lines from the details
        while let Some(last) = details.last() {
            if last.trim().is_empty() {
                details.pop();
            } else {
                break;
            }
        }

        Ok(LogfileSummary {
            command: command
                .context("Failed parsing error metrics: A command should be present")?,
            pid,
            parent_pid,
            details,
            log_path: make_relative(&self.root_dir, path),
            costs: CostsKind::None,
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
