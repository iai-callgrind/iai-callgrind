use std::collections::{BinaryHeap, HashMap};
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use log::debug;

use super::hashmap_parser::{CallgrindMap, HashMapParser, Id, RecordMember};
use super::model::{Costs, EventType};
use super::parser::{Parser, Sentinel};
use super::CallgrindOutput;
use crate::runner::callgrind::hashmap_parser::Record;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct FlamegraphMap(HashMap<String, Value>);

#[derive(Debug)]
pub struct FlamegraphParser {
    map: FlamegraphMap,
    project_root: PathBuf,
    sentinel: Option<Sentinel>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Value {
    inclusive_costs: Costs,
    exclusive_costs: Costs,
    is_inline: bool,
}

impl FlamegraphMap {
    pub fn insert(&mut self, id: String, value: Value) -> Option<Value> {
        self.0.insert(id, value)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    // Convert to stacks string format for this `EventType`
    //
    // # Errors
    //
    // If the event type was not present in the stacks
    pub fn to_stack_format(&self, event_type: &EventType) -> Result<Vec<String>> {
        #[derive(Debug, Eq)]
        struct HeapElem<'heap> {
            source: &'heap str,
            is_inline: bool,
            ex_cost: u64,
            in_cost: u64,
        }

        impl<'heap> Ord for HeapElem<'heap> {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                match self.in_cost.cmp(&other.in_cost) {
                    std::cmp::Ordering::Equal => self.source.cmp(other.source),
                    cmp => cmp.reverse(),
                }
            }
        }

        impl<'heap> PartialOrd for HeapElem<'heap> {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        impl<'heap> PartialEq for HeapElem<'heap> {
            fn eq(&self, other: &Self) -> bool {
                self.in_cost == other.in_cost && self.source == other.source
            }
        }

        let mut heap = BinaryHeap::new();
        for (
            id,
            Value {
                inclusive_costs,
                exclusive_costs,
                is_inline,
            },
        ) in &self.0
        {
            let in_cost = inclusive_costs.cost_by_type(event_type).ok_or_else(|| {
                anyhow!("Failed creating flamegraph stack: Missing event type '{event_type}'")
            })?;
            let ex_cost = exclusive_costs.cost_by_type(event_type).ok_or_else(|| {
                anyhow!("Failed creating flamegraph stack: Missing event type '{event_type}'")
            })?;
            heap.push(HeapElem {
                source: id,
                is_inline: *is_inline,
                ex_cost,
                in_cost,
            });
        }
        // dbg!(&heap);

        let mut stacks: Vec<String> = vec![];
        for HeapElem {
            ex_cost,
            source,
            is_inline,
            ..
        } in heap.into_sorted_vec()
        {
            if let Some(last) = stacks.last() {
                let (split, _) = last.rsplit_once(' ').unwrap();
                let stack = if is_inline {
                    format!("{split};[{source}] {ex_cost}")
                } else {
                    format!("{split};{source} {ex_cost}")
                };
                stacks.push(stack);
            } else if is_inline {
                stacks.push(format!("[{source}] {ex_cost}"));
            } else {
                stacks.push(format!("{source} {ex_cost}"));
            }
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
            map: FlamegraphMap::default(),
            project_root: project_root.into(),
        }
    }

    fn fold<'map>(&mut self, map: &'map CallgrindMap, key: &'map Id, value: &'map Record) {
        if self.map.get(&key.func).is_some() {
            return;
        }

        self.map.insert(
            key.func.clone(),
            Value {
                exclusive_costs: value.self_costs.clone(),
                inclusive_costs: value.inclusive_costs.clone(),
                is_inline: false,
            },
        );

        for member in &value.members {
            match member {
                RecordMember::Cfn(record) => {
                    let query = Id {
                        func: record.cfn.clone(),
                    };

                    let (cfn_key, cfn_value) = map
                        .get_key_value(&query)
                        .expect("A cfn record must have an fn record");
                    self.fold(map, cfn_key, cfn_value);
                }

                RecordMember::Inline(record) => {
                    if let Some(value) = record.fi.as_ref().or(record.fe.as_ref()) {
                        let path = PathBuf::from(&value);
                        let path = path.strip_prefix(&self.project_root).unwrap_or(&path);

                        self.map
                            .0
                            .entry(path.display().to_string())
                            .and_modify(|e| {
                                e.inclusive_costs.add(&record.costs);
                                e.exclusive_costs.add(&record.costs);
                            })
                            .or_insert(Value {
                                inclusive_costs: record.costs.clone(),
                                exclusive_costs: record.costs.clone(),
                                is_inline: true,
                            });
                    }
                }
            }
        }
    }
}

impl Parser for FlamegraphParser {
    type Output = FlamegraphMap;

    fn parse(mut self, output: &CallgrindOutput) -> Result<Self::Output> {
        debug!("Parsing flamegraph from file '{}'", output);

        let parser = HashMapParser {
            sentinel: self.sentinel.clone(),
        };

        let map = parser.parse(output)?;

        if map.is_empty() {
            return Ok(self.map);
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

        self.fold(&map, key, value);

        Ok(self.map)
    }
}
