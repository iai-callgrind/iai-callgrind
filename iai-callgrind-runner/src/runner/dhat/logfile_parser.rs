use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;

use crate::error::Error;
use crate::runner::callgrind::parser::Parser;
use crate::runner::tool::logfile_parser::LogfileSummary;
use crate::runner::tool::ToolOutputPath;
use crate::util::make_relative;

lazy_static! {
    static ref EXTRACT_FIELDS_RE: Regex =
        regex::Regex::new(r"^\s*==[0-9]+==\s*(?<key>.*?)\s*:\s*(?<value>.*)\s*$")
            .expect("Regex should compile");
    static ref EMPTY_LINE_RE: Regex =
        regex::Regex::new(r"^\s*==[0-9]+==\s*$").expect("Regex should compile");
    static ref FIXUP_NUMBERS_RE: Regex =
        regex::Regex::new(r"([0-9]),([0-9])").expect("Regex should compile");
    static ref EXTRACT_PID_RE: Regex =
        regex::Regex::new(r"^\s*==([0-9]+)==.*").expect("Regex should compile");
}

pub struct LogfileParser {
    pub root_dir: PathBuf,
}

#[derive(Debug)]
enum State {
    Header,
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
            .get(1)
            .expect("Log output should contain pid")
            .as_str()
            .parse::<i32>()
            .expect("Pid should be valid");

        let mut state = State::Header;
        let mut command = None;
        let mut fields = Vec::new();
        // TODO: FIX PARSING WHEN VERBOSE
        for line in iter.filter(|line| !EMPTY_LINE_RE.is_match(line)) {
            let caps = EXTRACT_FIELDS_RE.captures(&line);
            match (&state, caps) {
                (State::Header, Some(caps)) => {
                    let key = caps.name("key").unwrap().as_str();
                    if key.eq_ignore_ascii_case("command") {
                        let value = caps.name("value").unwrap().as_str();
                        command = Some(make_relative(&self.root_dir, value));
                        state = State::Body;
                    }
                }
                (State::Header, None) => continue,
                (State::Body, Some(caps)) => {
                    let key = caps.name("key").unwrap().as_str();
                    let value = caps.name("value").unwrap().as_str();
                    let value = FIXUP_NUMBERS_RE.replace_all(value, "$1$2");
                    fields.push((key.to_owned(), value.to_string()));
                }
                (State::Body, None) => break,
            }
        }

        Ok(LogfileSummary {
            command: command.expect("A command should be present"),
            pid,
            fields,
            body: vec![],
            error_summary: None,
            log_path: make_relative(&self.root_dir, path),
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
        debug!("DHAT: Parsing log file '{}'", log_path);

        let mut summaries = vec![];
        for path in log_path.real_paths() {
            let summary = self.parse_single(path)?;
            summaries.push(summary);
        }
        summaries.sort_by_key(|s| s.pid);
        Ok(summaries)
    }
}
