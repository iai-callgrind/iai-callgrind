use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::{Context, Result};

use super::logfile_parser::{parse_header, Logfile, LogfileParser, EMPTY_LINE_RE, STRIP_PREFIX_RE};
use crate::runner::summary::ToolMetrics;
use crate::util::make_relative;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    HeaderSpace,
    Body,
}

pub struct GenericLogfileParser {
    pub root_dir: PathBuf,
}

impl LogfileParser for GenericLogfileParser {
    fn parse_single(&self, path: PathBuf) -> Result<Logfile> {
        let file = File::open(&path)
            .with_context(|| format!("Error opening log file '{}'", path.display()))?;

        let mut iter = BufReader::new(file)
            .lines()
            .map(std::result::Result::unwrap)
            .skip_while(|l| l.trim().is_empty());

        let header = parse_header(&self.root_dir, &path, &mut iter)?;
        let mut details = vec![];

        let mut state = State::HeaderSpace;
        for line in iter {
            match &state {
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

        Ok(Logfile {
            header,
            details,
            path: make_relative(&self.root_dir, path),
            metrics: ToolMetrics::None,
        })
    }
}
