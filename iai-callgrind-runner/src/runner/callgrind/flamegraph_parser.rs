use std::path::PathBuf;

use super::hashmap_parser::{CallgrindMap, HashMapParser, Id, RecordMember};
use super::parser::{Parser, Sentinel};
use super::CallgrindOutput;
use crate::error::Result;
use crate::runner::callgrind::hashmap_parser::Record;
use crate::runner::flamegraph::{Stack, Stacks};

#[derive(Debug)]
pub struct FlamegraphParser {
    stacks: Stacks,
    project_root: PathBuf,
    sentinel: Option<Sentinel>,
}

impl FlamegraphParser {
    pub fn new<P>(sentinel: Option<&Sentinel>, project_root: P) -> Self
    where
        P: Into<PathBuf>,
    {
        Self {
            sentinel: sentinel.cloned(),
            stacks: Stacks::default(),
            project_root: project_root.into(),
        }
    }

    fn fold(&mut self, map: &CallgrindMap, key: &Id, value: &Record, last: Option<&Stack>) {
        self.stacks
            .add(&key.func, value.self_costs.clone(), false, last);

        if value.members.is_empty() {
            return;
        }

        // unwrap is fine here, because we just added a stack
        let last = self.stacks.last().cloned().unwrap();
        for member in &value.members {
            match member {
                RecordMember::Cfn(record) => {
                    let query = Id {
                        func: record.cfn.clone(),
                    };

                    let (cfn_key, cfn_value) = map
                        .get_key_value(&query)
                        .expect("A cfn record must have an fn record");

                    // Check for recursion. Inlined records are ignored.
                    if last.contains(&record.cfn, false) {
                        // Inclusive costs of recursive functions are meaningless, so do nothing
                    } else {
                        self.fold(map, cfn_key, cfn_value, Some(&last));
                    }
                }
                RecordMember::Inline(record) => {
                    if let Some(value) = record.fi.as_ref().or(record.fe.as_ref()) {
                        let path = PathBuf::from(&value);
                        let path = path.strip_prefix(&self.project_root).unwrap_or(&path);

                        self.stacks.add(
                            path.display().to_string(),
                            record.costs.clone(),
                            true,
                            Some(&last),
                        );
                    }
                }
            }
        }
    }
}

impl Parser for FlamegraphParser {
    type Output = Stacks;

    fn parse(mut self, output: &CallgrindOutput) -> Result<Self::Output> {
        let parser = HashMapParser {
            sentinel: self.sentinel.clone(),
        };

        let map = parser.parse(output)?;

        if map.is_empty() {
            return Ok(Stacks::default());
        }

        // Let's find our entry point which defaults to "main"
        let (key, value) = if let Some(key) = &map.sentinel_key {
            map.get_key_value(key)
                .expect("Resolved sentinel must be present in map")
        } else {
            map.iter()
                .find(|(k, _)| k.func == "main")
                .expect("'main' function must be present in callgrind output")
        };

        self.fold(&map, key, value, None);

        Ok(self.stacks)
    }
}
