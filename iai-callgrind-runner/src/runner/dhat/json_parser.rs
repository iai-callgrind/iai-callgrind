//! Module containing the json parser for dhat output files

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

use super::model::DhatData;
use super::tree::{RootTree, Tree};
use crate::api::EntryPoint;
use crate::runner::tool::logfile_parser;
use crate::runner::tool::parser::{Header, Parser, ParserOutput};
use crate::runner::tool::path::ToolOutputPath;
use crate::util::Glob;

/// The dhat output file json parser
pub struct JsonParser {
    entry_point: EntryPoint,
    frames: Vec<Glob>,
    output_path: ToolOutputPath,
}

impl JsonParser {
    /// Create a new `JsonParser`
    pub fn new(output_path: ToolOutputPath, entry_point: EntryPoint, frames: Vec<Glob>) -> Self {
        Self {
            entry_point,
            frames,
            output_path,
        }
    }
}

impl Parser for JsonParser {
    fn parse_single(&self, path: PathBuf) -> Result<ParserOutput> {
        let dhat_data = parse(&path)
            .with_context(|| format!("Error opening dhat output file '{}'", path.display()))?;

        let parent_pid = if let Some(logfile) = self.output_path.log_path_of(&path) {
            let file = File::open(&logfile)
                .with_context(|| format!("Error opening dhat log file '{}'", logfile.display()))?;

            let iter = BufReader::new(file)
                .lines()
                .map(std::result::Result::unwrap);
            let header = logfile_parser::parse_header(&logfile, iter)?;

            assert_eq!(
                header.pid, dhat_data.pid,
                "The pid of the json and log file should be equal"
            );

            header.parent_pid
        } else {
            None
        };

        let header = Header {
            command: dhat_data.command.clone(),
            pid: dhat_data.pid,
            parent_pid,
            thread: None,
            part: None,
            desc: vec![],
        };

        let tree = RootTree::from_json(dhat_data, &self.entry_point, &self.frames);

        Ok(ParserOutput {
            path,
            header,
            details: vec![],
            metrics: tree.metrics(),
        })
    }

    fn get_output_path(&self) -> &ToolOutputPath {
        &self.output_path
    }
}

// TODO: refactor: sort
/// Parse the dhat output file at `path` into [`DhatData`]
pub fn parse(path: &Path) -> Result<DhatData> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|error| {
        anyhow!(
            "Error parsing dhat output file '{}': {error}",
            path.display()
        )
    })
}
