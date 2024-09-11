use std::fs::File;
use std::io::{BufRead, BufReader};
use std::iter;
use std::path::PathBuf;

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::Regex;

use crate::error::Error;
use crate::runner::costs::Costs;
use crate::runner::summary::ToolRunSummary;
use crate::runner::tool::logfile_parser::{
    extract_pid, LogfileParser, LogfileSummary, EMPTY_LINE_RE, EXTRACT_FIELDS_RE, STRIP_PREFIX_RE,
};
use crate::util::make_relative;

// The different regex have to consider --time-stamp=yes
lazy_static! {
    static ref FIXUP_NUMBERS_RE: Regex =
        regex::Regex::new(r"([0-9]),([0-9])").expect("Regex should compile");
    static ref COSTS_RE: Regex =
        regex::Regex::new(r"([0-9]*) (\w*)(?: in ([0-9]*) (\w*))?").expect("Regex should compile");
}

pub struct DhatLogfileParser {
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
        if let Some(cap) = COSTS_RE.captures(value) {
            let c1 = cap.get(1).unwrap().as_str().parse().unwrap();
            let n1 = cap.get(2).unwrap().as_str();
            res.0.insert(format!("{field} {n1}"), c1);
            if let Some(blocks) = cap.get(3) {
                let c2 = blocks.as_str().parse().unwrap();
                let n2 = cap.get(4).unwrap().as_str();
                res.0.insert(format!("{field} {n2}"), c2);
            }
        }
    }
    res
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

        let costs = Some(fields_to_costs(&fields));
        Ok(LogfileSummary {
            command: command.expect("A command should be present"),
            pid,
            parent_pid,
            fields: vec![],
            details,
            error_summary: None,
            log_path: make_relative(&self.root_dir, path),
            costs,
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
