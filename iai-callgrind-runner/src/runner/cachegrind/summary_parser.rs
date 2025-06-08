use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use anyhow::Result;
use log::{debug, trace};

use super::parser::parse_header;
use crate::error::Error;
use crate::runner::summary::ToolMetrics;
use crate::runner::tool::logfile_parser::{self, Header, Parser};
use crate::runner::tool::ToolOutputPath;

/// TODO: UPDATE DOCS
/// Parse the `total:` line in the callgrind output or `summary:` if total is not present
///
/// The format is described [here](https://valgrind.org/docs/manual/cl-format.html)
///
/// The summary would usually be the first choice, but there are bugs in the summary line if
/// callgrind client requests are used, which is why the total is used as primary metric and then
/// the summary.
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
/// ```text
/// Spec from https://valgrind.org/docs/manual/cg-manual.html
/// file         ::= desc_line* cmd_line events_line data_line+ summary_line
/// desc_line    ::= "desc:" ws? non_nl_string
/// cmd_line     ::= "cmd:" ws? cmd
/// events_line  ::= "events:" ws? (event ws)+
/// data_line    ::= file_line | fn_line | count_line
/// file_line    ::= "fl=" filename
/// fn_line      ::= "fn=" fn_name
/// count_line   ::= line_num (ws+ count)* ws*
/// summary_line ::= "summary:" ws? count (ws+ count)+ ws*
/// count        ::= num
/// ```
#[derive(Debug)]
pub struct SummaryParser {
    pub output_path: ToolOutputPath,
}

impl Parser for SummaryParser {
    fn parse_single(&self, path: PathBuf) -> Result<logfile_parser::ParserResult> {
        debug!(
            "Parsing cachegrind output file '{}' for the summary",
            path.display()
        );

        let mut iter = BufReader::new(File::open(&path)?)
            .lines()
            .map(Result::unwrap);

        let properties = parse_header(&mut iter)
            .map_err(|error| Error::ParseError((path.clone(), error.to_string())))?;

        let mut metrics = None;
        for line in iter {
            if let Some(suffix) = line.strip_prefix("summary:") {
                trace!("Found line with summary: '{line}'");

                let mut inner = properties.metrics_prototype.clone();
                inner.add_iter_str(suffix.split_ascii_whitespace())?;
                metrics = Some(inner);

                trace!("Updated counters to '{:?}'", &metrics);
            }
        }

        if let Some(metrics) = metrics {
            let header = Header {
                command: properties.cmd,
                // TODO: PARSE THE path FOR THE PID
                pid: 0,
                // TODO: PARSE THE LOGFILE FOR THE PPID
                parent_pid: None,
                desc: properties.desc,
            };
            Ok(logfile_parser::ParserResult {
                path,
                header,
                details: vec![],
                metrics: ToolMetrics::CachegrindMetrics(metrics),
            })
        } else {
            Err(Error::ParseError((path.clone(), "No summary line found".to_owned())).into())
        }
    }

    fn get_output_path(&self) -> &ToolOutputPath {
        &self.output_path
    }
}
