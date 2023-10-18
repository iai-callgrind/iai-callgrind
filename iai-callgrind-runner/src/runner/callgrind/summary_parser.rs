use anyhow::Result;
use log::{debug, trace};

use super::parser::Parser;
use super::{CallgrindOutput, CallgrindStats};
use crate::error::Error;
use crate::runner::callgrind::parser::parse_header;

pub struct SummaryParser;

impl Parser for SummaryParser {
    type Output = CallgrindStats;

    fn parse(&self, output: &CallgrindOutput) -> Result<Self::Output>
    where
        Self: std::marker::Sized,
    {
        debug!(
            "Parsing callgrind output file '{}' for a summary or totals",
            output
        );

        let mut iter = output.lines()?;
        let config = parse_header(&mut iter)
            .map_err(|error| Error::ParseError((output.0.clone(), error.to_string())))?;

        let mut found = false;
        let mut costs = config.costs_prototype;
        for line in iter {
            if let Some(stripped) = line.strip_prefix("summary:") {
                trace!("Found line with summary: '{}'", line);
                costs.add_iter_str(stripped.split_ascii_whitespace());
                trace!("Updated counters to '{:?}'", &costs);
                found = true;
                break;
            }

            if let Some(stripped) = line.strip_prefix("totals:") {
                trace!("Found line with totals: '{}'", line);
                costs.add_iter_str(stripped.split_ascii_whitespace());
                trace!("Updated counters to '{:?}'", &costs);
                found = true;
                break;
            }
        }

        if found {
            Ok(CallgrindStats(costs))
        } else {
            Err(Error::ParseError((
                output.as_path().to_owned(),
                "No summary or totals line found".to_owned(),
            ))
            .into())
        }
    }
}
