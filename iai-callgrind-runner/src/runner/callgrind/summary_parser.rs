use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use anyhow::Result;
use log::{debug, trace};

use super::model::Costs;
use super::parser::{parse_header, CallgrindProperties};
use crate::error::Error;
use crate::runner::tool::{Parser, ToolOutputPath};

pub struct SummaryParser;

impl Parser for SummaryParser {
    type Output = Costs;

    fn parse(&self, output_path: &ToolOutputPath) -> Result<Self::Output>
    where
        Self: std::marker::Sized,
    {
        debug!(
            "Parsing callgrind output file '{}' for a summary or totals",
            output_path
        );

        let mut iter = output_path.lines()?;
        let config = parse_header(&mut iter)
            .map_err(|error| Error::ParseError((output_path.to_path(), error.to_string())))?;

        let mut found = false;
        let mut costs = config.costs_prototype;
        for line in iter {
            if let Some(stripped) = line.strip_prefix("summary:") {
                trace!("Found line with summary: '{}'", line);
                costs.add_iter_str(stripped.split_ascii_whitespace());
                if costs.iter().all(|(_c, u)| *u == 0) {
                    trace!(
                        "Continuing file processing as summary indicates \"client_request\" are \
                         used."
                    );
                    continue;
                };
                trace!("Updated counters to '{:?}'", &costs);
                found = true;
                break;
            }

            // TODO: Scan to end of file
            if let Some(stripped) = line.strip_prefix("totals:") {
                trace!("Found line with totals: '{}'", line);
                costs.add_iter_str(stripped.split_ascii_whitespace());
                trace!("Updated counters to '{:?}'", &costs);
                found = true;
                break;
            }
        }

        if found {
            Ok(costs)
        } else {
            Err(Error::ParseError((
                output_path.to_path(),
                "No summary or totals line found".to_owned(),
            ))
            .into())
        }
    }

    fn parse_single_alt(&self, path: &Path) -> Result<(CallgrindProperties, Self::Output)> {
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
        let mut costs = properties.costs_prototype.clone();
        for line in iter {
            if let Some(stripped) = line.strip_prefix("summary:") {
                trace!("Found line with summary: '{}'", line);
                costs.add_iter_str(stripped.split_ascii_whitespace());
                if costs.iter().all(|(_c, u)| *u == 0) {
                    trace!(
                        "Continuing file processing as summary indicates \"client_request\" are \
                         used."
                    );
                    continue;
                };
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
            Ok((properties, costs))
        } else {
            Err(Error::ParseError((
                path.to_owned(),
                "No summary or totals line found".to_owned(),
            ))
            .into())
        }
    }
}
