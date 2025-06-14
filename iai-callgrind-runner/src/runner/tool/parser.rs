use std::cmp::Ordering;
use std::path::PathBuf;

use anyhow::Result;
use log::debug;

use super::ToolOutputPath;
use crate::runner::summary::ToolMetrics;

pub trait Parser {
    fn parse_single(&self, path: PathBuf) -> Result<ParserOutput>;
    /// Return a sorted vector of parser results
    fn parse_with(&self, output_path: &ToolOutputPath) -> Result<Vec<ParserOutput>> {
        debug!("{}: Parsing file '{}'", output_path.tool.id(), output_path);
        let Ok(paths) = output_path.real_paths() else {
            return Ok(vec![]);
        };

        let mut parser_results = Vec::with_capacity(paths.len());
        for path in paths {
            let parsed = self.parse_single(path)?;
            let position = parser_results
                .binary_search_by(|probe: &ParserOutput| probe.compare_target_ids(&parsed))
                .unwrap_or_else(|e| e);

            parser_results.insert(position, parsed);
        }

        Ok(parser_results)
    }

    // TODO: RETURN Error if not at least 1 file could be parsed?
    fn parse(&self) -> Result<Vec<ParserOutput>> {
        self.parse_with(self.get_output_path())
    }

    fn parse_base(&self) -> Result<Vec<ParserOutput>> {
        self.parse_with(&self.get_output_path().to_base_path())
    }

    fn get_output_path(&self) -> &ToolOutputPath;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub command: String,
    pub pid: i32,
    pub parent_pid: Option<i32>,
    pub thread: Option<usize>,
    pub part: Option<u64>,
    pub desc: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParserOutput {
    pub path: PathBuf,
    pub header: Header,
    pub details: Vec<String>,
    pub metrics: ToolMetrics,
}

impl ParserOutput {
    /// Compare by target ids `pid`, `part` and `thread`
    ///
    /// Same as in [`CallgrindProperties::compare_target_ids`]
    ///
    /// Highest precedence takes `pid`. Second is `part` and third is `thread` all sorted ascending.
    /// See also [Callgrind Format](https://valgrind.org/docs/manual/cl-format.html#cl-format.reference.grammar)
    pub fn compare_target_ids(&self, other: &Self) -> Ordering {
        self.header.pid.cmp(&other.header.pid).then_with(|| {
            self.header
                .thread
                .cmp(&other.header.thread)
                .then_with(|| self.header.part.cmp(&other.header.part))
        })
    }
}
