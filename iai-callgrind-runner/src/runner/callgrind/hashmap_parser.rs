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
    pub file: Option<PathBuf>,
    pub func: String,
}

// TODO: TRY TO GIVE IT A BETTER NAME
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Value {
    pub costs: Costs,
    pub obj_path: Option<PathBuf>,
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
    fn parse(self, output: &CallgrindOutput) -> Result<Self::Output> {
        // Ignore 'cob'
        #[derive(Debug, Default)]
        struct CfnRecord {
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

        // TODO: CLEANUP REDUNDANT VARS
        let mut curr_obj = None;
        let mut curr_file = None;
        let mut curr_fn = None;
        let mut curr_name: Option<Id> = None;

        let mut cfn_record = None;

        let mut curr_fn_costs = config.costs_prototype.clone();

        // These are global in the callgrind_annotate source !!
        let mut cfn_totals = HashMap::<Id, Costs>::new();
        let mut fn_totals = HashMap::<Id, Costs>::new();
        let mut obj_name = HashMap::<Id, PathBuf>::new();

        let mut sentinel_key = None;

        // We start within the header
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
                        fn_totals.insert(id, curr_fn_costs);
                    }

                    // Create new
                    curr_fn = Some(func.to_owned());
                    let tmp = Id {
                        file: curr_file.as_ref().cloned(),
                        func: func.to_owned(),
                    };
                    curr_obj
                        .as_ref()
                        .and_then(|o: &PathBuf| obj_name.insert(tmp.clone(), o.clone()));
                    curr_fn_costs = fn_totals
                        .get(&tmp)
                        .unwrap_or(&config.costs_prototype)
                        .clone();
                    curr_name = Some(tmp);
                }
                Some(("fi" | "fe", inline)) => {
                    fn_totals
                        .entry(curr_name.expect("Valid fi/fe line"))
                        .and_modify(|e| *e = curr_fn_costs.clone())
                        .or_insert(curr_fn_costs);
                    curr_file = Some(make_path(&self.project_root, inline));
                    curr_name = Some(Id {
                        file: curr_file.as_ref().cloned(),
                        func: curr_fn.as_ref().unwrap().clone(),
                    });
                    curr_fn_costs = fn_totals
                        .get(curr_name.as_ref().unwrap())
                        .unwrap_or(&config.costs_prototype)
                        .clone();
                }
                Some(("cfi" | "cfl", inline)) => {
                    let record = cfn_record.get_or_insert(CfnRecord::default());
                    record.file = Some(make_path(&self.project_root, inline));
                }
                Some(("cfn", cfn)) => {
                    let record = cfn_record.get_or_insert(CfnRecord::default());
                    record.name = Some(record.file.as_ref().map_or_else(
                        || Id {
                            func: cfn.to_owned(),
                            file: curr_file.as_ref().map(std::borrow::ToOwned::to_owned),
                        },
                        |f| Id {
                            func: cfn.to_owned(),
                            file: Some(PathBuf::from(f)),
                        },
                    ));
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
                            .and_modify(|e| e.add(&costs))
                            .or_insert(costs);
                    }
                }
                Some(("cob" | "jump" | "jcnd" | "jfi" | "jfn", _)) => {
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
                file: curr_file,
                func: curr_fn.take().unwrap(),
            };
            fn_totals.insert(id, curr_fn_costs);
        }

        // Correct inclusive totals
        for (key, value) in cfn_totals {
            fn_totals.insert(key, value);
        }
        let map = fn_totals
            .into_iter()
            .map(|(id, costs)| {
                let obj_path = obj_name.get(&id).cloned();
                (id, Value { costs, obj_path })
            })
            .collect();

        Ok(CallgrindMap {
            map,
            sentinel: self.sentinel.clone(),
            sentinel_key,
        })
    }
}
