use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::{anyhow, Result};

use super::json_model::Dhat;

pub fn parse_json(path: &Path) -> Result<Dhat> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    serde_json::from_reader(reader).map_err(|error| {
        anyhow!(
            "Error parsing dhat output file '{}': {error}",
            path.display()
        )
    })
}
