use std::path::PathBuf;

pub mod format;
pub mod logfile_parser;

#[derive(Debug, Clone)]
pub struct LogfileSummary {
    pub command: PathBuf,
    pub pid: i32,
    pub fields: Vec<(String, String)>,
}
