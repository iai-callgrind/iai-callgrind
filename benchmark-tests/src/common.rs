use std::fs::File;
use std::path::Path;

use anyhow::{anyhow, Result};
use iai_callgrind_runner::runner::summary::BenchmarkSummary;
use iai_callgrind_runner::util::EitherOrBoth;

#[derive(Debug)]
pub struct Summary(pub BenchmarkSummary);

impl Summary {
    pub fn new(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        serde_json::from_reader(file)
            .map(Self)
            .map_err(|error| anyhow!(error))
    }

    pub fn get_name(&self) -> String {
        if let Some(id) = self.0.id.as_ref() {
            format!("{} {id}", self.0.module_path)
        } else {
            self.0.module_path.to_string()
        }
    }

    #[track_caller]
    pub fn assert_costs_not_all_zero(&self) {
        if let Some(callgrind_summary) = &self.0.callgrind_summary {
            for summary in callgrind_summary
                .summaries
                .summaries
                .iter()
                .map(|s| &s.events)
                .chain(std::iter::once(&callgrind_summary.summaries.total.total))
            {
                match summary.extract_costs() {
                    EitherOrBoth::Left(new_costs) => {
                        assert!(
                            !new_costs.0.iter().all(|(_, c)| *c == 0),
                            "All *new* costs were zero for '{}'",
                            self.get_name()
                        );
                    }
                    EitherOrBoth::Right(old_costs) => {
                        assert!(
                            !old_costs.0.iter().all(|(_, c)| *c == 0),
                            "All *old* costs were zero for '{}'",
                            self.get_name()
                        );
                    }
                    EitherOrBoth::Both((new_costs, old_costs)) => {
                        assert!(
                            !new_costs.0.iter().all(|(_, c)| *c == 0),
                            "All *new* costs were zero for '{}'",
                            self.get_name()
                        );
                        assert!(
                            !old_costs.0.iter().all(|(_, c)| *c == 0),
                            "All *old* costs were zero for '{}'",
                            self.get_name()
                        );
                    }
                }
            }
        }
    }
}
