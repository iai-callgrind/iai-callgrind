use std::collections::BinaryHeap;
use std::fmt::Write;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use log::debug;

use super::hashmap_parser::{CallgrindMap, HashMapParser};
use super::model::EventType;
use super::parser::{Parser, Sentinel};
use super::CallgrindOutput;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct FlamegraphMap(CallgrindMap);

#[derive(Debug)]
pub struct FlamegraphParser {
    project_root: PathBuf,
    sentinel: Option<Sentinel>,
}

impl FlamegraphMap {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    // Convert to stacks string format for this `EventType`
    //
    // # Errors
    //
    // If the event type was not present in the stacks
    #[allow(clippy::too_many_lines)]
    pub fn to_stack_format(&self, event_type: &EventType) -> Result<Vec<String>> {
        #[derive(Debug, Eq)]
        struct HeapElem {
            source: String,
            cost: u64,
            obj: Option<PathBuf>,
        }

        impl Ord for HeapElem {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                match self.cost.cmp(&other.cost) {
                    std::cmp::Ordering::Equal => self.source.cmp(&other.source),
                    cmp => cmp.reverse(),
                }
            }
        }

        impl PartialOrd for HeapElem {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl PartialEq for HeapElem {
            fn eq(&self, other: &Self) -> bool {
                self.cost == other.cost && self.source == other.source
            }
        }

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
                .cost_by_type(event_type)
                .ok_or_else(|| {
                    anyhow!("Failed creating flamegraph stack: Missing event type '{event_type}'")
                })?
        } else {
            self.0
                .map
                .iter()
                .find(|(k, _)| k.func == "main")
                .expect("'main' function must be present in callgrind output")
                .1
                .costs
                .cost_by_type(event_type)
                .ok_or_else(|| {
                    anyhow!("Failed creating flamegraph stack: Missing event type '{event_type}'")
                })?
        };

        let mut heap = BinaryHeap::new();
        for (id, value) in &self.0.map {
            let cost = value.costs.cost_by_type(event_type).ok_or_else(|| {
                anyhow!("Failed creating flamegraph stack: Missing event type '{event_type}'")
            })?;
            if cost <= reference_cost {
                let source = if let Some(file) = &id.file {
                    let display = file.display().to_string();
                    if display == "???" {
                        id.func.clone()
                    } else {
                        format!("{display}:{}", id.func)
                    }
                } else {
                    id.func.clone()
                };
                heap.push(HeapElem {
                    source,
                    cost,
                    obj: value.obj_path.clone(),
                });
            }
        }

        let mut stacks: Vec<String> = vec![];
        let len = heap.len();
        if len > 1 {
            for window in heap.into_sorted_vec().windows(2) {
                // There is only the slice size of 2 possible due to the windows size of 2
                if let [h1, h2] = window {
                    let mut stack = if let Some(last) = stacks.last() {
                        let (split, _) = last.rsplit_once(' ').unwrap();
                        format!("{split};{}", h1.source)
                    } else {
                        h1.source.clone()
                    };
                    if let Some(obj) = &h1.obj {
                        write!(stack, " [{}]", obj.display()).unwrap();
                    }
                    write!(stack, " {}", h1.cost - h2.cost).unwrap();

                    stacks.push(stack);

                    // The last window needs to push the last element too
                    if stacks.len() == len - 1 {
                        let last = stacks.last().unwrap();
                        let (split, _) = last.rsplit_once(' ').unwrap();

                        let mut stack = format!("{split};{}", h2.source);
                        if let Some(obj) = &h2.obj {
                            write!(stack, " [{}]", obj.display()).unwrap();
                        }
                        write!(stack, " {}", h2.cost).unwrap();

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

    fn parse(self, output: &CallgrindOutput) -> Result<Self::Output> {
        debug!("Parsing flamegraph from file '{}'", output);

        let parser = HashMapParser {
            project_root: self.project_root,
            sentinel: self.sentinel.clone(),
        };

        parser.parse(output).map(FlamegraphMap)
    }
}
