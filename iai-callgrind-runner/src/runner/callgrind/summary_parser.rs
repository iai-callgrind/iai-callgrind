use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::Result;
use log::{debug, trace};

use super::model::Metrics;
use super::parser::{parse_header, CallgrindParser, CallgrindProperties};
use crate::error::Error;

/// Parse the `summary:` line in the callgrind output or `total:` if summary is not present
///
/// The format is described [here](https://valgrind.org/docs/manual/cl-format.html)
///
/// Regarding the summary:
///
/// For the visualization to be able to show cost percentage, a sum of the cost of the full run has
/// to be known. Usually, it is assumed that this is the sum of all cost lines in a file. But
/// sometimes, this is not correct. The "summary:" line in the header gives the full cost for the
/// profile run.
///
/// This header line specifies a summary cost, which should be equal or larger than a total over all
/// self costs. It may be larger as the cost lines may not represent all cost of the program run.
#[derive(Debug)]
pub struct SummaryParser;

impl CallgrindParser for SummaryParser {
    type Output = Metrics;

    fn parse_single(&self, path: &Path) -> Result<(CallgrindProperties, Self::Output)> {
        debug!(
            "Parsing callgrind output file '{}' for a summary or totals",
            path.display()
        );

        let mut iter = BufReader::new(File::open(path)?)
            .lines()
            .map(Result::unwrap);
        let properties = parse_header(&mut iter)
            .map_err(|error| Error::ParseError((path.to_owned(), error.to_string())))?;

        let mut found = false;
        let mut metrics = properties.metrics_prototype.clone();
        for line in iter {
            if let Some(suffix) = line.strip_prefix("summary:") {
                trace!("Found line with summary: '{}'", line);
                metrics.add_iter_str(suffix.split_ascii_whitespace())?;
                if metrics.iter().all(|(_c, u)| *u == 0) {
                    trace!(
                        "Continuing file processing as summary indicates \"client_request\" are \
                         used."
                    );
                    continue;
                };
                trace!("Updated counters to '{:?}'", &metrics);
                found = true;
                break;
            }

            if let Some(suffix) = line.strip_prefix("totals:") {
                trace!("Found line with totals: '{}'", line);
                metrics.add_iter_str(suffix.split_ascii_whitespace())?;
                trace!("Updated counters to '{:?}'", &metrics);
                found = true;
                break;
            }
        }

        if found {
            Ok((properties, metrics))
        } else {
            Err(Error::ParseError((
                path.to_owned(),
                "No summary or totals line found".to_owned(),
            ))
            .into())
        }
    }
}
