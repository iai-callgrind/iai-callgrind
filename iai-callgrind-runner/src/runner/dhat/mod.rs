use std::path::PathBuf;

pub mod format;
pub mod logfile_parser;

#[derive(Debug, Clone)]
pub struct LogfileSummary {
    command: PathBuf,
    pid: i32,
    fields: Vec<(String, String)>,
}
