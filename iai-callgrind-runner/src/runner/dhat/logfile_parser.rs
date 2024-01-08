use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;

use crate::error::Error;
use crate::runner::costs::Costs;
use crate::runner::summary::CostsSummary;
use crate::runner::tool::logfile_parser::LogfileSummary;
use crate::runner::tool::{Parser, ToolOutputPath};
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
    static ref FIXUP_NUMBERS_RE: Regex =
        regex::Regex::new(r"([0-9]),([0-9])").expect("Regex should compile");
    static ref BYTES_RE: Regex =
        regex::Regex::new(r"([0-9]*) bytes(?: in ([0-9]*) blocks)?").expect("Regex should compile");
}

pub struct LogfileParser {
    pub root_dir: PathBuf,
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Header,
    HeaderSpace,
    Body,
    Fields,
    Footer,
}

fn fields_to_costs(fields: &[(String, String)]) -> Costs<String> {
    let mut res = Costs::with_event_kinds([]);
    for (field, value) in fields {
        if let Some(cap) = BYTES_RE.captures(value) {
            let bytes = cap.get(1).unwrap().as_str().parse().unwrap();
            res.0.insert(format!("{field} bytes"), bytes);
            if let Some(blocks) = cap.get(2) {
                let blocks = blocks.as_str().parse().unwrap();
                res.0.insert(format!("{field} blocks"), blocks);
            }
        }
    }
    res
}

impl LogfileParser {
    fn parse_single(&self, path: PathBuf) -> Result<(LogfileSummary, Costs<String>)> {
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
        let mut fields = vec![];
        let mut details = vec![];
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

                        // Total: ... is the first line of the fields we're interested in
                        if key.to_ascii_lowercase().as_str() == "total" {
                            let value = caps.name("value").unwrap().as_str();
                            let value = FIXUP_NUMBERS_RE.replace_all(value, "$1$2");
                            fields.push((key.to_owned(), value.to_string()));

                            state = State::Fields;
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
                State::Fields => {
                    if let Some(caps) = EXTRACT_FIELDS_RE.captures(&line) {
                        let key = caps.name("key").unwrap().as_str();
                        let value = caps.name("value").unwrap().as_str();
                        let value = FIXUP_NUMBERS_RE.replace_all(value, "$1$2");
                        fields.push((key.to_owned(), value.to_string()));
                    } else {
                        state = State::Footer;
                    }
                }
                State::Footer => break,
            }
        }

        while let Some(last) = details.last() {
            if last.trim().is_empty() {
                details.pop();
            } else {
                break;
            }
        }

        let costs = fields_to_costs(&fields);

        Ok((
            LogfileSummary {
                command: command.expect("A command should be present"),
                pid,
                parent_pid,
                fields,
                details,
                error_summary: None,
                log_path: make_relative(&self.root_dir, path),
                cost_summary: None,
            },
            costs,
        ))
    }
}

impl Parser for LogfileParser {
    type Output = Vec<LogfileSummary>;

    fn parse(&self, output_path: &ToolOutputPath) -> Result<Self::Output>
    where
        Self: std::marker::Sized,
    {
        let log_path = output_path.to_log_output();
        debug!("DHAT: Parsing log file '{}'", log_path);

        let mut summaries = vec![];
        let paths = log_path.real_paths()?;
        if let Ok(old_paths) = log_path.to_base_path().real_paths() {
            for (path, old_path) in paths.into_iter().zip(old_paths) {
                let (mut summary, costs) = self.parse_single(path)?;
                let (_, old_costs) = self.parse_single(old_path)?;
                summary.cost_summary = Some(CostsSummary::new(&costs, Some(&old_costs)));
                summaries.push(summary)
            }
        } else {
            for path in log_path.real_paths()? {
                let (mut summary, costs) = self.parse_single(path)?;
                summary.cost_summary = Some(CostsSummary::new(&costs, None));
                summaries.push(summary)
            }
        }
        summaries.sort_by_key(|s| s.pid);
        Ok(summaries)
    }
}
