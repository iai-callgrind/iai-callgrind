use std::path::PathBuf;

use log::trace;

use super::hashmap_parser::{CallgrindMap, HashMapParser, Id, RecordMember};
use super::parser::CallgrindParser;
use super::{CallgrindOutput, Sentinel};
use crate::error::Result;
use crate::runner::callgrind::hashmap_parser::Record;

#[derive(Debug)]
pub struct FlamegraphParser {
    stacks: Vec<String>,
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
            stacks: Vec::default(),
            project_root: project_root.into(),
        }
    }

    fn fold(
        &mut self,
        map: &CallgrindMap,
        key: &Id,
        value: &Record,
        // WRAP &STR INTO Stack struct
        last: &str,
    ) {
        let this = if last.is_empty() {
            key.func.clone()
        } else {
            format!("{last};{}", key.func)
        };

        // TODO: MAKE the choice of counts configurable. Currently uses only instruction counts.
        // Adjust the name for the counts in such cases.
        // TODO: MAKE the choice of a title for the svg files configurable??
        // TODO: MAKE the choice of a name for the counts configurable??
        let stack = format!("{this} {}", value.self_costs.cost_by_index(0).unwrap());
        trace!("Pushing stack: {}", &stack);
        self.stacks.push(stack);

        for member in &value.members {
            match member {
                RecordMember::Cfn(record) => {
                    let query = Id {
                        func: record.cfn.clone(),
                    };

                    let (cfn_key, cfn_value) = map
                        .get_key_value(&query)
                        .expect("A cfn record must have an fn record");

                    // TODO: What about nested recursion? A>B>A etc. This only detects A>A
                    if cfn_key == key {
                        // Inclusive costs of recursive functions are meaningless, so do nothing
                    } else {
                        self.fold(map, cfn_key, cfn_value, &this);
                    }
                }
                RecordMember::Inline(record) => {
                    if let Some(value) = record.fi.as_ref().or(record.fe.as_ref()) {
                        let path = PathBuf::from(&value);
                        let path = path.strip_prefix(&self.project_root).unwrap_or(&path);

                        let stack = format!(
                            "{this};[{}] {}",
                            path.display(),
                            record.costs.cost_by_index(0).unwrap()
                        );
                        trace!("Pushing stack: {}", &stack);
                        self.stacks.push(stack);
                    }
                }
            }
        }
    }
}

impl CallgrindParser for FlamegraphParser {
    type Output = Vec<String>;

    fn parse(mut self, output: &CallgrindOutput) -> Result<Self::Output> {
        let parser = HashMapParser {
            sentinel: self.sentinel.clone(),
            ..Default::default()
        };

        let map = parser.parse(output)?;

        if map.is_empty() {
            return Ok(vec![]);
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

        self.fold(&map, key, value, "");

        Ok(self.stacks)
    }
}
