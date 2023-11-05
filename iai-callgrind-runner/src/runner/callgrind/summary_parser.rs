use anyhow::Result;
use log::{debug, trace};

use super::model::Costs;
use super::parser::Parser;
use crate::error::Error;
use crate::runner::callgrind::parser::parse_header;
use crate::runner::tool::ToolOutputPath;

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
            .map_err(|error| Error::ParseError((output_path.path.clone(), error.to_string())))?;

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
            Ok(costs)
        } else {
            Err(Error::ParseError((
                output_path.as_path().to_owned(),
                "No summary or totals line found".to_owned(),
            ))
            .into())
        }
    }
}
