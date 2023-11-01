use std::collections::BinaryHeap;
use std::fmt::Write;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use log::debug;

use super::hashmap_parser::{CallgrindMap, HashMapParser};
use super::parser::{Parser, Sentinel};
use crate::api::EventKind;
use crate::runner::callgrind::hashmap_parser::SourcePath;
use crate::runner::common::ToolOutput;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct FlamegraphMap(CallgrindMap);

#[derive(Debug)]
pub struct FlamegraphParser {
    project_root: PathBuf,
    sentinel: Option<Sentinel>,
}

#[derive(Debug, Eq, PartialEq)]
struct HeapElem {
    source: String,
    cost: u64,
}

impl FlamegraphMap {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn make_summary(&mut self) -> Result<()> {
        for value in self.0.map.values_mut() {
            value
                .costs
                .make_summary()
                .map_err(|error| anyhow!("Failed calculating summary events: {error}"))?;
        }
        Ok(())
    }

    // Convert to stacks string format for this `EventType`
    //
    // # Errors
    //
    // If the event type was not present in the stacks
    #[allow(clippy::too_many_lines)]
    pub fn to_stack_format(&self, event_kind: &EventKind) -> Result<Vec<String>> {
        if self.0.map.is_empty() {
            return Ok(vec![]);
        }

        // Let's find our entry point which defaults to "main"
        let reference_cost = if let Some(key) = &self.0.sentinel_key {
            self.0
                .map
                .get(key)
                .expect("Resolved sentinel must be present in map")
                .costs
                .cost_by_kind(event_kind)
                .ok_or_else(|| {
                    anyhow!("Failed creating flamegraph stack: Missing event type '{event_kind}'")
                })?
        } else {
            self.0
                .map
                .iter()
                .find(|(k, _)| k.func == "main")
                .expect("'main' function must be present in callgrind output")
                .1
                .costs
                .cost_by_kind(event_kind)
                .ok_or_else(|| {
                    anyhow!("Failed creating flamegraph stack: Missing event type '{event_kind}'")
                })?
        };

        let mut heap = BinaryHeap::new();
        for (id, value) in &self.0.map {
            let cost = value.costs.cost_by_kind(event_kind).ok_or_else(|| {
                anyhow!("Failed creating flamegraph stack: Missing event type '{event_kind}'")
            })?;
            if cost <= reference_cost {
                let mut source = String::new();
                if let Some(file) = &id.file {
                    match file {
                        SourcePath::Unknown => write!(source, "{}", id.func).unwrap(),
                        SourcePath::Rust(path)
                        | SourcePath::Relative(path)
                        | SourcePath::Absolute(path) => {
                            write!(source, "{}:{}", path.display(), id.func).unwrap();
                        }
                    }
                } else {
                    write!(source, "{}", id.func).unwrap();
                };
                if let Some(path) = &id.obj {
                    match path {
                        SourcePath::Unknown => {}
                        SourcePath::Rust(path)
                        | SourcePath::Relative(path)
                        | SourcePath::Absolute(path) => {
                            write!(source, " [{}]", path.display()).unwrap();
                        }
                    }
                }
                heap.push(HeapElem { source, cost });
            }
        }

        let mut stacks: Vec<String> = vec![];
        let len = heap.len();
        if len > 1 {
            for window in heap.into_sorted_vec().windows(2) {
                // There is only the slice size of 2 possible due to the windows size of 2
                if let [h1, h2] = window {
                    let stack = if let Some(last) = stacks.last() {
                        let (split, _) = last.rsplit_once(' ').unwrap();
                        format!("{split};{} {}", h1.source, h1.cost - h2.cost)
                    } else {
                        format!("{} {}", h1.source, h1.cost - h2.cost)
                    };

                    stacks.push(stack);

                    // The last window needs to push the last element too
                    if stacks.len() == len - 1 {
                        let last = stacks.last().unwrap();
                        let (split, _) = last.rsplit_once(' ').unwrap();

                        let stack = format!("{split};{} {}", h2.source, h2.cost);
                        stacks.push(stack);
                    }
                }
            }
        } else {
            // unwrap is safe since heap.len() == 1 here
            let elem = heap.pop().unwrap();
            stacks.push(format!("{} {}", elem.source, elem.cost));
        }
        Ok(stacks)
    }
}

impl FlamegraphParser {
    pub fn new<P>(sentinel: Option<&Sentinel>, project_root: P) -> Self
    where
        P: Into<PathBuf>,
    {
        Self {
            sentinel: sentinel.cloned(),
            project_root: project_root.into(),
        }
    }
}

impl Parser for FlamegraphParser {
    type Output = FlamegraphMap;

    fn parse(&self, output: &ToolOutput) -> Result<Self::Output> {
        debug!("Parsing flamegraph from file '{}'", output);

        let parser = HashMapParser {
            project_root: self.project_root.clone(),
            sentinel: self.sentinel.clone(),
        };

        parser.parse(output).map(FlamegraphMap)
    }
}

impl Ord for HeapElem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cost
            .cmp(&other.cost)
            .reverse()
            .then_with(|| self.source.cmp(&other.source))
    }
}

impl PartialOrd for HeapElem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
