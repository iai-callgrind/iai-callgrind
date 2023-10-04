use std::path::PathBuf;

use log::trace;

use super::parser::{Parser, Sentinel};
use super::{CallgrindOutput, CallgrindStats};
use crate::error::{Error, Result};
use crate::runner::callgrind::parser::{parse_header, PositionsMode};

pub struct SentinelParser {
    sentinel: Sentinel,
    bench_file: PathBuf,
}

impl SentinelParser {
    pub fn new<T>(sentinel: &Sentinel, bench_file: T) -> Self
    where
        T: Into<PathBuf>,
    {
        Self {
            sentinel: sentinel.clone(),
            bench_file: bench_file.into(),
        }
    }
}

impl Parser for SentinelParser {
    type Output = CallgrindStats;

    fn parse(self, output: &CallgrindOutput) -> Result<Self::Output>
    where
        Self: std::marker::Sized,
    {
        trace!(
            "Parsing callgrind output file '{}' for '{}'",
            output,
            self.sentinel
        );

        trace!(
            "Using sentinel: '{}' for file name ending with: '{}'",
            &self.sentinel,
            self.bench_file.display()
        );

        let mut iter = output.lines()?;
        let config = parse_header(&mut iter)
            .map_err(|message| Error::ParseError((output.path.clone(), message)))?;

        let mut costs = config.costs_prototype;
        let mode = config.positions_mode;

        let mut start_record = false;
        // TODO: It's not needed to parse the whole file if the sentinel is a fn= method which is
        // unique in the whole file.
        for line in iter {
            let line = line.trim_start();
            if line.is_empty() {
                start_record = false;
            }
            if !start_record {
                if line.starts_with("fl=") && line.ends_with(self.bench_file.to_str().unwrap()) {
                    trace!("Found line with benchmark file: '{}'", line);
                } else if line.starts_with(&self.sentinel.to_fn()) {
                    trace!("Found line with sentinel: '{}'", line);
                    start_record = true;
                } else {
                    // do nothing
                }
                continue;
            }

            // we check if it is a line with counters and summarize them
            if line.starts_with(|c: char| c.is_ascii_digit()) {
                // From the documentation of the callgrind format:
                // > If a cost line specifies less event counts than given in the "events" line, the
                // > rest is assumed to be zero.
                trace!("Found line with counters: '{}'", line);
                costs.add_iter_str(line
                    .split_ascii_whitespace()
                    // skip the first number which is just the line number or instr number or in
                    // case of `instr line` skip 2
                    .skip(if mode == PositionsMode::InstrLine { 2 } else { 1 }));
                trace!("Updated counters to '{:?}'", &costs);
            } else {
                trace!("Skipping line: '{}'", line);
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
