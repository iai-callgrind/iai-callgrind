use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use lazy_static::lazy_static;
use regex::Regex;

use super::logfile_parser::{
    parse_header, Logfile, LogfileParser, EMPTY_LINE_RE, EXTRACT_FIELDS_RE, STRIP_PREFIX_RE,
};
use crate::api::ErrorMetricKind;
use crate::runner::metrics::Metrics;
use crate::runner::summary::ToolMetrics;
use crate::util::make_relative;

lazy_static! {
    static ref EXTRACT_ERROR_SUMMARY_RE: Regex = regex::Regex::new(
        r"^.*(?<errs>[0-9]+).*(?<ctxs>[0-9]+).*(?<s_errs>[0-9]+).*(?<s_ctxs>[0-9]+).*$"
    )
    .expect("Regex should compile");
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    HeaderSpace,
    Body,
}

pub struct ErrorMetricLogfileParser {
    pub root_dir: PathBuf,
}

impl LogfileParser for ErrorMetricLogfileParser {
    fn parse_single(&self, path: PathBuf) -> Result<Logfile> {
        let file = File::open(&path)
            .with_context(|| format!("Error opening log file '{}'", path.display()))?;

        let mut iter = BufReader::new(file)
            .lines()
            .map(std::result::Result::unwrap)
            .skip_while(|l| l.trim().is_empty());

        let header = parse_header(&self.root_dir, &path, &mut iter)?;

        let metrics_prototype = Metrics::from_iter([
            ErrorMetricKind::Errors,
            ErrorMetricKind::Contexts,
            ErrorMetricKind::SuppressedErrors,
            ErrorMetricKind::SuppressedContexts,
        ]);

        let mut details = vec![];
        let mut metrics = None;

        let mut state = State::HeaderSpace;
        for line in iter {
            match &state {
                State::HeaderSpace if EMPTY_LINE_RE.is_match(&line) => {}
                State::HeaderSpace | State::Body => {
                    if state == State::HeaderSpace {
                        state = State::Body;
                    }

                    // TODO: THIS could be improved and match the EXTRACT_ERROR_SUMMARY_RE directly
                    // stripping the prefix first if possible
                    if let Some(caps) = EXTRACT_FIELDS_RE.captures(&line) {
                        let key = caps.name("key").unwrap().as_str();

                        if key.eq_ignore_ascii_case("error summary") {
                            let error_summary_value = caps.name("value").unwrap().as_str();

                            let caps = EXTRACT_ERROR_SUMMARY_RE
                                .captures(error_summary_value)
                                .ok_or(anyhow!(
                                    "Failed to extract error summary from string".to_owned()
                                ))?;

                            // There might be multiple `ERROR SUMMARY` lines. We only use the last
                            let mut new_metrics: Metrics<ErrorMetricKind> =
                                metrics_prototype.clone();
                            new_metrics.add_iter_str([
                                caps.name("errs").unwrap().as_str(),
                                caps.name("ctxs").unwrap().as_str(),
                                caps.name("s_errs").unwrap().as_str(),
                                caps.name("s_ctxs").unwrap().as_str(),
                            ]);

                            metrics = Some(new_metrics);
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

        Ok(Logfile {
            header,
            path: make_relative(&self.root_dir, path),
            metrics: ToolMetrics::ErrorMetrics(metrics.context(
                "Failed collecting error metrics: An error summary line should be present",
            )?),
            details,
        })
    }
}
