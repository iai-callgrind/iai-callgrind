use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::{Context, Result};
use log::debug;

use super::LogfileSummary;
use crate::error::Error;
use crate::runner::callgrind::parser::Parser;
use crate::runner::tool::ToolOutputPath;

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
            .ok_or_else(|| Error::ParseError((path, "Empty file".to_owned())))?;
        let extract_pid_re =
            regex::Regex::new(r"^\s*==([0-9]+)==.*").expect("Regex should compile");
        let pid = extract_pid_re
            .captures(line.trim())
            .expect("Log output should not be malformed")
            .get(1)
            .expect("Log output should contain pid")
            .as_str()
            .parse::<i32>()
            .expect("Pid should be valid");

        let extract_fields_re =
            regex::Regex::new(r"^\s*==[0-9]+==\s*(?<key>.*?)\s*:\s*(?<value>.*)\s*$")
                .expect("Regex should compile");
        let empty_line_re = regex::Regex::new(r"^\s*==[0-9]+==\s*$").expect("Regex should compile");
        let fixup_numbers_re = regex::Regex::new(r"([0-9]),([0-9])").expect("Regex should compile");

        let mut state = State::Header;
        let mut command = None;
        let mut fields = Vec::new();
        for line in iter.filter(|line| !empty_line_re.is_match(line)) {
            let caps = extract_fields_re.captures(&line);
            match (&state, caps) {
                (State::Header, Some(caps)) => {
                    let key = caps.name("key").unwrap().as_str();
                    if key.eq_ignore_ascii_case("command") {
                        let value = caps.name("value").unwrap().as_str();
                        let path = PathBuf::from(value);
                        command = if let Ok(relative) = path.strip_prefix(&self.root_dir) {
                            Some(relative.to_owned())
                        } else {
                            Some(path)
                        };
                        state = State::Body;
                    }
                }
                (State::Header, None) => continue,
                (State::Body, Some(caps)) => {
                    let key = caps.name("key").unwrap().as_str();
                    let value = caps.name("value").unwrap().as_str();
                    let value = fixup_numbers_re.replace_all(value, "$1$2");
                    fields.push((key.to_owned(), value.to_string()));
                }
                (State::Body, None) => break,
            }
        }

        Ok(LogfileSummary {
            command: command.expect("A command should be present"),
            pid,
            fields,
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
