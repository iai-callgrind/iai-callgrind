use std::fs::File;
use std::path::Path;

use anyhow::{anyhow, Result};
use iai_callgrind_runner::runner::summary::BenchmarkSummary;

#[derive(Debug)]
pub struct Summary(pub BenchmarkSummary);

impl Summary {
    pub fn new(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        serde_json::from_reader(file)
            .map(Self)
            .map_err(|error| anyhow!(error))
    }

    #[track_caller]
    pub fn assert_not_zero(&self) {
        if let Some(callgrind_summary) = &self.0.callgrind_summary {
            for summary in &callgrind_summary.summaries {
                let (new_costs, old_costs) = summary.events.extract_costs();
                if let Some(new_costs) = new_costs {
                    assert!(!new_costs.0.iter().all(|(_, c)| *c == 0));
                }
                if let Some(old_costs) = old_costs {
                    assert!(!old_costs.0.iter().all(|(_, c)| *c == 0));
                }
            }
        }
    }
}
