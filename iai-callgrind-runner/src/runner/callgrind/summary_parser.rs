use log::trace;

use super::parser::Parser;
use super::{CallgrindOutput, CallgrindStats};
use crate::error::{Error, Result};
use crate::runner::callgrind::parser::parse_header;

pub struct SummaryParser;

impl Parser for SummaryParser {
    type Output = CallgrindStats;

    fn parse(self, output: &CallgrindOutput) -> Result<Self::Output>
    where
        Self: std::marker::Sized,
    {
        trace!(
            "Parsing callgrind output file '{}' for a summary or totals",
            output
        );

        let mut iter = output.lines()?;
        let config = parse_header(&mut iter)
            .map_err(|message| Error::ParseError((output.path.clone(), message)))?;

        let mut costs = config.costs_prototype;
        for line in iter {
            if let Some(stripped) = line.strip_prefix("summary:") {
                trace!("Found line with summary: '{}'", line);
                costs.add_iter_str(stripped.split_ascii_whitespace());
                trace!("Updated counters to '{:?}'", &costs);
                break;
            }
            // TODO: If the summary line doesn't exist use the HashMapParser instead and then
            // sum up the (self?) costs of each Record.
            if let Some(stripped) = line.strip_prefix("totals:") {
                trace!("Found line with totals: '{}'", line);
                costs.add_iter_str(stripped.split_ascii_whitespace());
                trace!("Updated counters to '{:?}'", &costs);
                break;
            }
        }

        Ok(CallgrindStats {
            instructions_executed: costs.cost_by_index(0).unwrap(),
            total_data_cache_reads: costs.cost_by_index(1).unwrap(),
            total_data_cache_writes: costs.cost_by_index(2).unwrap(),
            l1_instructions_cache_read_misses: costs.cost_by_index(3).unwrap(),
            l1_data_cache_read_misses: costs.cost_by_index(4).unwrap(),
            l1_data_cache_write_misses: costs.cost_by_index(5).unwrap(),
            l3_instructions_cache_read_misses: costs.cost_by_index(6).unwrap(),
            l3_data_cache_read_misses: costs.cost_by_index(7).unwrap(),
            l3_data_cache_write_misses: costs.cost_by_index(8).unwrap(),
        })
    }
}
