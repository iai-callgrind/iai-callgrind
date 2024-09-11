use std::fs::File;
use std::io::{BufRead, BufReader};
use std::iter;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;

use crate::api::DhatMetricKind;
use crate::error::Error;
use crate::runner::costs::Costs;
use crate::runner::summary::{CostsKind, ToolRunSummary};
use crate::runner::tool::logfile_parser::{
    extract_pid, LogfileParser, LogfileSummary, EMPTY_LINE_RE, EXTRACT_FIELDS_RE, STRIP_PREFIX_RE,
};
use crate::util::make_relative;

// The different regex have to consider --time-stamp=yes
lazy_static! {
    static ref FIXUP_NUMBERS_RE: Regex =
        regex::Regex::new(r"([0-9]),([0-9])").expect("Regex should compile");
    static ref COSTS_RE: Regex =
        regex::Regex::new(r"^\s*(?<bytes>[0-9]+)\s*bytes(?:\s*in\s*(?<blocks>[0-9]+))?.*$")
            .expect("Regex should compile");
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Header,
    HeaderSpace,
    Body,
    Fields,
    Footer,
}

fn parse_line(
    line: &str,
    root_dir: &Path,
    state: &mut State,
    command: &mut Option<PathBuf>,
    costs: &mut Costs<DhatMetricKind>,
    details: &mut Vec<String>,
    parent_pid: &mut Option<i32>,
) -> Result<bool> {
    match &state {
        State::Header if !EMPTY_LINE_RE.is_match(line) => {
            if let Some(caps) = EXTRACT_FIELDS_RE.captures(line) {
                let key = caps.name("key").unwrap().as_str();
                match key.to_ascii_lowercase().as_str() {
                    "command" => {
                        let value = caps.name("value").unwrap().as_str();
                        *command = Some(make_relative(root_dir, value));
                    }
                    "parent pid" => {
                        let value = caps.name("value").unwrap().as_str().to_owned();
                        *parent_pid = Some(
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
        State::Header => *state = State::HeaderSpace,
        State::HeaderSpace if EMPTY_LINE_RE.is_match(line) => {}
        State::HeaderSpace | State::Body => {
            if *state == State::HeaderSpace {
                *state = State::Body;
            }

            if let Some(caps) = EXTRACT_FIELDS_RE.captures(line) {
                let key = caps.name("key").unwrap().as_str();

                // Total: ... is the first line of the fields we're interested in
                if key.to_ascii_lowercase().as_str() == "total" {
                    *state = State::Fields;
                    return parse_line(line, root_dir, state, command, costs, details, parent_pid);
                }
            }

            if let Some(caps) = STRIP_PREFIX_RE.captures(line) {
                let rest_of_line = caps.name("rest").unwrap().as_str();

                details.push(rest_of_line.to_owned());
            } else {
                details.push(line.to_owned());
            }
        }
        State::Fields => {
            // The original metrics lines look like this:
            //
            // ==2960865== Total:     156,362 bytes in 78 blocks
            // ==2960865== At t-gmax: 48,821 bytes in 13 blocks
            // ==2960865== At t-end:  0 bytes in 0 blocks
            // ==2960865== Reads:     119,827 bytes
            // ==2960865== Writes:    136,997 bytes
            //
            // The prefix with the pid can be different but the `EXTRACT_FIELDS_RE` takes
            // care of that.
            //
            // The metric lines with bytes and blocks need to be parsed into two separate
            // metric kinds
            if let Some(fields_caps) = EXTRACT_FIELDS_RE.captures(line) {
                let key = fields_caps.name("key").unwrap().as_str();
                let value = fields_caps.name("value").unwrap().as_str();
                let value = FIXUP_NUMBERS_RE.replace_all(value, "$1$2");

                if let Some(costs_caps) = COSTS_RE.captures(&value) {
                    let num_bytes = costs_caps.name("bytes").unwrap().as_str().parse()?;
                    let num_blocks = costs_caps
                        .name("blocks")
                        .and_then(|s| s.as_str().parse().ok());

                    match key {
                        "Total" => {
                            costs.0.insert(DhatMetricKind::TotalBytes, num_bytes);
                            costs.0.insert(
                                DhatMetricKind::TotalBlocks,
                                num_blocks.ok_or_else(|| anyhow!("Error parsing blocks"))?,
                            );
                        }
                        "At t-gmax" => {
                            costs.0.insert(DhatMetricKind::AtTGmaxBytes, num_bytes);
                            costs.0.insert(
                                DhatMetricKind::AtTGmaxBlocks,
                                num_blocks.ok_or_else(|| anyhow!("Error parsing blocks"))?,
                            );
                        }
                        "At t-end" => {
                            costs.0.insert(DhatMetricKind::AtTEndBytes, num_bytes);
                            costs.0.insert(
                                DhatMetricKind::AtTEndBlocks,
                                num_blocks.ok_or_else(|| anyhow!("Error parsing blocks"))?,
                            );
                        }
                        "Reads" => {
                            let metric_kind = DhatMetricKind::ReadsBytes;
                            costs.0.insert(metric_kind, num_bytes);
                        }
                        "Writes" => {
                            let metric_kind = DhatMetricKind::WritesBytes;
                            costs.0.insert(metric_kind, num_bytes);
                        }
                        _ => {
                            debug!("Ignoring invalid dhat metric kind: {key}");
                        }
                    }
                }
            } else {
                *state = State::Footer;
            }
        }
        State::Footer => {
            return Ok(false);
        }
    }

    Ok(true)
}

pub struct DhatLogfileParser {
    pub root_dir: PathBuf,
}

impl LogfileParser for DhatLogfileParser {
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

        let mut state = State::Header;
        let mut command = None;
        let mut costs = Costs::empty();
        let mut details = vec![];
        let mut parent_pid = None;
        for line in iter {
            if !parse_line(
                &line,
                &self.root_dir,
                &mut state,
                &mut command,
                &mut costs,
                &mut details,
                &mut parent_pid,
            )? {
                break;
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
            fields: vec![],
            details,
            error_summary: None,
            log_path: make_relative(&self.root_dir, path),
            costs: CostsKind::DhatCosts(costs),
        })
    }

    fn merge_logfile_summaries(
        &self,
        old: Vec<LogfileSummary>,
        new: Vec<LogfileSummary>,
    ) -> Vec<ToolRunSummary> {
        let old = old.into_iter().map(Some).chain(iter::repeat_with(|| None));
        let new = new.into_iter().map(Some).chain(iter::repeat_with(|| None));
        let zip = iter::zip(old, new).take_while(|(o, n)| o.is_some() || n.is_some());

        let mut res = vec![];
        for (old, new) in zip {
            match (old, new) {
                (None, None) => unreachable!(),
                (Some(old), None) => res.push(old.old_into_tool_run()),
                (None, Some(new)) => res.push(new.new_into_tool_run()),
                (Some(old), Some(new)) => {
                    if old.command == new.command {
                        res.push(new.merge(&old));
                    } else {
                        res.push(old.old_into_tool_run());
                        res.push(new.new_into_tool_run());
                    }
                }
            }
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    // ==2960865== Total:     156,362 bytes in 78 blocks
    // ==2960865== At t-gmax: 48,821 bytes in 13 blocks
    // ==2960865== At t-end:  0 bytes in 0 blocks
    // ==2960865== Reads:     119,827 bytes
    // ==2960865== Writes:    136,997 bytes
    #[rstest]
    #[case::some_bytes_in_blocks("156362 bytes in 78 blocks")]
    #[case::zero_bytes_in_blocks("0 bytes in 0 blocks")]
    #[case::some_bytes("156362 bytes")]
    #[case::zero_bytes("0 bytes")]
    fn test_costs_re_when_match(#[case] haystack: &str) {
        assert!(COSTS_RE.is_match(haystack));
    }
}
