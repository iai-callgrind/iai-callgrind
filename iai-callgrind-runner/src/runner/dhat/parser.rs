use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::{anyhow, Result};

use super::model::DhatData;

pub fn parse_json(path: &Path) -> Result<DhatData> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|error| {
        anyhow!(
            "Error parsing dhat output file '{}': {error}",
            path.display()
        )
    })
}
