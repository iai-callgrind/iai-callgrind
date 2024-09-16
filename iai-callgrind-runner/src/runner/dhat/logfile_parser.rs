use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;

use crate::api::DhatMetricKind;
use crate::runner::costs::Costs;
use crate::runner::summary::CostsKind;
use crate::runner::tool::logfile_parser::{
    parse_header, Logfile, LogfileParser, EMPTY_LINE_RE, EXTRACT_FIELDS_RE, STRIP_PREFIX_RE,
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
    HeaderSpace,
    Body,
    Fields,
    Footer,
}

pub struct DhatLogfileParser {
    pub root_dir: PathBuf,
}

impl DhatLogfileParser {
    /// Parse a single line of the logfile
    ///
    /// A return value of `false` indicates parsing is complete.
    fn parse_line(
        line: &str,
        state: &mut State,
        costs: &mut Costs<DhatMetricKind>,
        details: &mut Vec<String>,
    ) -> Result<bool> {
        match &state {
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
                        return DhatLogfileParser::parse_line(line, state, costs, details);
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
                                costs.insert(DhatMetricKind::TotalBytes, num_bytes);
                                costs.insert(
                                    DhatMetricKind::TotalBlocks,
                                    num_blocks.ok_or_else(|| anyhow!("Error parsing blocks"))?,
                                );
                            }
                            "At t-gmax" => {
                                costs.insert(DhatMetricKind::AtTGmaxBytes, num_bytes);
                                costs.insert(
                                    DhatMetricKind::AtTGmaxBlocks,
                                    num_blocks.ok_or_else(|| anyhow!("Error parsing blocks"))?,
                                );
                            }
                            "At t-end" => {
                                costs.insert(DhatMetricKind::AtTEndBytes, num_bytes);
                                costs.insert(
                                    DhatMetricKind::AtTEndBlocks,
                                    num_blocks.ok_or_else(|| anyhow!("Error parsing blocks"))?,
                                );
                            }
                            "Reads" => {
                                let metric_kind = DhatMetricKind::ReadsBytes;
                                costs.insert(metric_kind, num_bytes);
                            }
                            "Writes" => {
                                let metric_kind = DhatMetricKind::WritesBytes;
                                costs.insert(metric_kind, num_bytes);
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
}

impl LogfileParser for DhatLogfileParser {
    fn parse_single(&self, path: PathBuf) -> Result<Logfile> {
        let file = File::open(&path)
            .with_context(|| format!("Error opening log file '{}'", path.display()))?;

        let mut iter = BufReader::new(file)
            .lines()
            .map(std::result::Result::unwrap)
            .skip_while(|l| l.trim().is_empty());

        let header = parse_header(&self.root_dir, &path, &mut iter)?;

        let mut costs = Costs::empty();
        let mut details = vec![];

        let mut state = State::HeaderSpace;
        for line in iter {
            if !DhatLogfileParser::parse_line(&line, &mut state, &mut costs, &mut details)? {
                break;
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
            path: make_relative(&self.root_dir, path),
            details,
            costs: CostsKind::DhatCosts(costs),
        })
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::some_bytes_in_blocks("156362 bytes in 78 blocks")]
    #[case::zero_bytes_in_blocks("0 bytes in 0 blocks")]
    #[case::some_bytes("156362 bytes")]
    #[case::zero_bytes("0 bytes")]
    fn test_costs_re_when_match(#[case] haystack: &str) {
        assert!(COSTS_RE.is_match(haystack));
    }
}
