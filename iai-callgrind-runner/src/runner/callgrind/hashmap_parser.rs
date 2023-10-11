use std::collections::hash_map::Iter;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use log::trace;
use serde::{Deserialize, Serialize};

use super::model::Costs;
use super::parser::{parse_header, Parser, Sentinel};
use super::CallgrindOutput;
use crate::error::Error;

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallgrindMap {
    pub map: HashMap<Id, Value>,
    pub sentinel: Option<Sentinel>,
    pub sentinel_key: Option<Id>,
}

/// Parse a callgrind outfile into a `HashMap`
///
/// This parser is a based on `callgrind_annotate` and how the summarize the function inclusive
/// costs.
#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashMapParser {
    pub sentinel: Option<Sentinel>,
    pub project_root: PathBuf,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Id {
    pub obj: Option<PathBuf>,
    pub file: Option<PathBuf>,
    pub func: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Value {
    pub costs: Costs,
}

impl CallgrindMap {
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn iter(&self) -> Iter<'_, Id, Value> {
        self.map.iter()
    }

    pub fn get_key_value(&self, k: &Id) -> Option<(&Id, &Value)> {
        self.map.get_key_value(k)
    }
}

impl Parser for HashMapParser {
    type Output = CallgrindMap;

    #[allow(clippy::too_many_lines)]
    #[allow(clippy::similar_names)]
    fn parse(&self, output: &CallgrindOutput) -> Result<Self::Output> {
        // Ignore 'cob'
        #[derive(Debug, Default)]
        struct CfnRecord {
            obj: Option<PathBuf>,
            file: Option<PathBuf>,
            name: Option<Id>,
            calls: u64,
        }

        fn make_path(root: &Path, source: &str) -> PathBuf {
            let path = PathBuf::from(source);
            if let Ok(stripped) = path.strip_prefix(root) {
                stripped.to_owned()
            } else {
                path
            }
        }

        let mut iter = output.lines()?;
        let config = parse_header(&mut iter)
            .map_err(|error| Error::ParseError((output.0.clone(), error.to_string())))?;

        let mut curr_obj = None;
        let mut curr_file = None;
        let mut curr_fn = None;
        let mut curr_name: Option<Id> = None;

        let mut cfn_record = None;

        let mut curr_fn_costs = config.costs_prototype.clone();

        // These are global in the callgrind_annotate source !!
        let mut cfn_totals = HashMap::<Id, Value>::new();
        let mut fn_totals = HashMap::<Id, Value>::new();

        let mut sentinel_key = None;

        // We start within he header
        let mut is_header = true;
        for line in iter {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // The first line which can be split around '=' is a non header line
            let split = if is_header {
                if let Some(split) = line.split_once('=') {
                    is_header = false;
                    Some(split)
                } else {
                    continue;
                }
            } else {
                line.split_once('=')
            };

            match split {
                Some(("ob", obj)) => {
                    curr_obj = Some(make_path(&self.project_root, obj));
                }
                Some(("fl", file)) => {
                    curr_file = Some(make_path(&self.project_root, file));
                }
                Some(("fn", func)) => {
                    // Commit result
                    if let Some(id) = curr_name.take() {
                        if self
                            .sentinel
                            .as_ref()
                            .map_or(false, |sentinel| sentinel.matches(&id.func))
                        {
                            trace!("Found sentinel: {}", &id.func);
                            sentinel_key = Some(id.clone());
                        }
                        fn_totals.insert(
                            id,
                            Value {
                                costs: curr_fn_costs,
                            },
                        );
                    }

                    // Create new
                    curr_fn = Some(func.to_owned());
                    let tmp = Id {
                        file: curr_file.as_ref().cloned(),
                        func: func.to_owned(),
                        obj: curr_obj.clone(),
                    };
                    curr_fn_costs = fn_totals
                        .get(&tmp)
                        .map_or(&config.costs_prototype, |value| &value.costs)
                        .clone();
                    curr_name = Some(tmp);
                }
                Some(("fi" | "fe", inline)) => {
                    fn_totals
                        .entry(curr_name.expect("Valid fi/fe line"))
                        .and_modify(|e| e.costs = curr_fn_costs.clone())
                        .or_insert(Value {
                            costs: curr_fn_costs,
                        });
                    curr_file = Some(make_path(&self.project_root, inline));
                    curr_name = Some(Id {
                        obj: curr_obj.as_ref().cloned(),
                        file: curr_file.as_ref().cloned(),
                        func: curr_fn.as_ref().unwrap().clone(),
                    });
                    curr_fn_costs = fn_totals
                        .get(curr_name.as_ref().unwrap())
                        .map_or(&config.costs_prototype, |value| &value.costs)
                        .clone();
                }
                Some(("cob", cob)) => {
                    let record = cfn_record.get_or_insert(CfnRecord::default());
                    record.obj = Some(make_path(&self.project_root, cob));
                }
                Some(("cfi" | "cfl", inline)) => {
                    let record = cfn_record.get_or_insert(CfnRecord::default());
                    record.file = Some(make_path(&self.project_root, inline));
                }
                Some(("cfn", cfn)) => {
                    let record = cfn_record.get_or_insert(CfnRecord::default());
                    record.name = Some(Id {
                        obj: record.obj.as_ref().or(curr_obj.as_ref()).cloned(),
                        func: cfn.to_owned(),
                        file: record.file.as_ref().or(curr_file.as_ref()).cloned(),
                    });
                }
                Some(("calls", calls)) => {
                    let record = cfn_record.as_mut().unwrap();
                    record.calls = calls
                        .split_ascii_whitespace()
                        .take(1)
                        .map(|s| s.parse::<u64>().unwrap())
                        .sum();
                }
                None if line.starts_with(|c: char| c.is_ascii_digit()) => {
                    let mut costs = config.costs_prototype.clone();
                    costs.add_iter_str(
                        line.split_whitespace()
                            .skip(config.positions_prototype.len()),
                    );

                    curr_fn_costs.add(&costs);
                    if let Some(cfn_record) = cfn_record.take() {
                        cfn_totals
                            .entry(
                                cfn_record
                                    .name
                                    .as_ref()
                                    .expect("cfn entry must be present at this point")
                                    .clone(),
                            )
                            .and_modify(|value| value.costs.add(&costs))
                            .or_insert(Value { costs });
                    }
                }
                Some(("jump" | "jcnd" | "jfi" | "jfn", _)) => {
                    // we ignore these
                }
                None if line.starts_with("totals:") || line.starts_with("summary:") => {
                    // we ignore these
                }
                Some(_) | None => panic!("Malformed line: '{line}'"),
            }
        }

        // Finish up if we actually found records past the header
        if !is_header {
            let id = Id {
                obj: curr_obj,
                file: curr_file,
                func: curr_fn.take().unwrap(),
            };
            fn_totals.insert(
                id,
                Value {
                    costs: curr_fn_costs,
                },
            );
        }

        // Correct inclusive totals
        for (key, value) in cfn_totals {
            fn_totals.insert(key, value);
        }

        Ok(CallgrindMap {
            map: fn_totals,
            sentinel: self.sentinel.clone(),
            sentinel_key,
        })
    }
}
