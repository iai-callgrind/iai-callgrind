use std::path::Path;

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use regex::Regex;

use super::parser::{Header, ParserOutput};
use crate::error::Error;
use crate::runner::summary::ToolMetricSummary;
use crate::util::EitherOrBoth;

// The different regex have to consider --time-stamp=yes
lazy_static! {
    pub static ref EXTRACT_FIELDS_RE: Regex = regex::Regex::new(
        r"^\s*(==|--)([0-9:.]+\s+)?[0-9]+(==|--)\s*(?<key>.*?)\s*:\s*(?<value>.*)\s*$"
    )
    .expect("Regex should compile");
    pub static ref EMPTY_LINE_RE: Regex =
        regex::Regex::new(r"^\s*(==|--)([0-9:.]+\s+)?[0-9]+(==|--)\s*$")
            .expect("Regex should compile");
    pub static ref STRIP_PREFIX_RE: Regex =
        regex::Regex::new(r"^\s*(==|--)([0-9:.]+\s+)?[0-9]+(==|--) (?<rest>.*)$")
            .expect("Regex should compile");
    static ref EXTRACT_PID_RE: Regex =
        regex::Regex::new(r"^\s*(==|--)([0-9:.]+\s+)?(?<pid>[0-9]+)(==|--).*")
            .expect("Regex should compile");
}

#[derive(Debug, Clone, PartialEq)]
pub struct LogfileSummary {
    pub logfile: EitherOrBoth<ParserOutput>,
    pub metrics_summary: ToolMetricSummary,
}

pub fn extract_pid(line: &str) -> Result<i32> {
    EXTRACT_PID_RE
        .captures(line.trim())
        .context("Log output should not be malformed")?
        .name("pid")
        .context("Log output should contain pid")?
        .as_str()
        .parse::<i32>()
        .context("Pid should be valid")
}

/// Parse the logfile header
///
/// The logfile header is the same for all tools
pub fn parse_header(path: &Path, mut lines: impl Iterator<Item = String>) -> Result<Header> {
    let next = lines.next();

    let (pid, next) = if let Some(next) = next {
        (extract_pid(&next)?, next)
    } else {
        return Err(Error::ParseError(path.to_owned(), "Empty file".to_owned()).into());
    };

    let mut parent_pid = None;
    let mut command = None;
    for line in std::iter::once(next).chain(lines) {
        if EMPTY_LINE_RE.is_match(&line) {
            // The header is separated from the body by at least one empty line. The first
            // empty line is removed from the iterator.
            break;
        } else if let Some(caps) = EXTRACT_FIELDS_RE.captures(&line) {
            let key = caps.name("key").unwrap().as_str();

            // These unwraps are safe. If there is a key, there is also a value present
            match key.to_ascii_lowercase().as_str() {
                "command" => {
                    let value = caps.name("value").unwrap().as_str();
                    command = Some(value.to_owned());
                }
                "parent pid" => {
                    let value = caps.name("value").unwrap().as_str().to_owned();
                    parent_pid = Some(
                        value
                            .as_str()
                            .parse::<i32>()
                            .context("Failed parsing log file: Parent pid should be valid")?,
                    );
                }
                _ => {
                    // Ignore other header lines
                }
            }
        } else {
            // Some malformed header line which we ignore
        }
    }

    Ok(Header {
        command: command.with_context(|| {
            format!(
                "Error parsing header of logfile '{}': A command should be present",
                path.display()
            )
        })?,
        pid,
        parent_pid,
        thread: None,
        part: None,
        desc: vec![],
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::equals_sign(
        "==1746070== Cachegrind, a high-precision tracing profiler",
        1_746_070_i32
    )]
    #[case::hyphen(
        "--1746070-- warning: L3 cache found, using its data for the LL simulation.",
        1_746_070_i32
    )]
    #[case::timestamp(
        "==00:00:00:00.000 1811497== Callgrind, a call-graph generating cache profiler",
        1_811_497_i32
    )]
    fn test_extract_pid(#[case] haystack: &str, #[case] expected: i32) {
        let actual = extract_pid(haystack).unwrap();
        assert_eq!(actual, expected);
    }
}
