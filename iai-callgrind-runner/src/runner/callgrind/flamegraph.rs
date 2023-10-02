use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use inferno::flamegraph::Options;
use log::{trace, warn};

use super::hashmap_parser::{HashMapParser, Id};
use super::{CallgrindOutput, CallgrindParser, Sentinel};
use crate::error::{IaiCallgrindError, Result};
use crate::runner::callgrind::hashmap_parser::Record;

#[derive(Debug)]
pub struct CallgrindFlamegraph {
    stacks: Vec<String>,
    project_root: PathBuf,
    sentinel: Option<Sentinel>,
    title: String,
}

impl CallgrindFlamegraph {
    pub fn new<T, P>(title: T, sentinel: Option<&Sentinel>, project_root: P) -> Self
    where
        T: Into<String>,
        P: Into<PathBuf>,
    {
        Self {
            title: title.into(),
            sentinel: sentinel.cloned(),
            stacks: Vec::default(),
            project_root: project_root.into(),
        }
    }

    fn fold(
        &mut self,
        map: &HashMap<Id, Record>,
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
        let stack = format!("{this} {}", value.self_costs.get_by_index(0).unwrap().cost);
        trace!("Pushing stack: {}", &stack);
        self.stacks.push(stack);

        for inline in &value.inlines {
            if let Some(value) = inline.fi.as_ref().or(inline.fe.as_ref()) {
                let path = PathBuf::from(&value);
                let path = path.strip_prefix(&self.project_root).unwrap_or(&path);

                let stack = format!(
                    "{this};[{}] {}",
                    path.display(),
                    inline.costs.get_by_index(0).unwrap().cost
                );
                trace!("Pushing stack: {}", &stack);
                self.stacks.push(stack);
            }
        }

        for cfn in &value.cfns {
            let query = Id {
                func: cfn.cfn.clone(),
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
    }

    pub fn create<T>(&self, dest: T) -> Result<()>
    where
        T: AsRef<Path>,
    {
        if self.stacks.is_empty() {
            warn!("Unable to create a flamegraph: Callgrind didn't record any events");
            return Ok(());
        }

        let output_file = File::create(dest).map_err(|error| {
            IaiCallgrindError::Other(format!("Creating flamegraph file failed: {error}"))
        })?;

        let mut options = Options::default();
        options.count_name = "Instructions".to_owned();
        options.title = self.title.clone();

        inferno::flamegraph::from_lines(
            &mut options,
            self.stacks.iter().map(std::string::String::as_str),
            output_file,
        )
        .map_err(|error| {
            crate::error::IaiCallgrindError::Other(format!(
                "Creating flamegraph file failed: {error}"
            ))
        })
    }
}

impl CallgrindParser for CallgrindFlamegraph {
    fn parse<T>(&mut self, output: T) -> Result<()>
    where
        T: AsRef<CallgrindOutput>,
    {
        let output = output.as_ref();

        let mut parser = HashMapParser {
            sentinel: self.sentinel.clone(),
            ..Default::default()
        };
        parser.parse(output)?;
        let map = parser.map;

        if map.is_empty() {
            return Ok(());
        }

        // Let's find our entry point which defaults to "main"
        let (key, value) = if let Some(key) = parser.sentinel_key {
            map.get_key_value(&key)
                .expect("Resolved sentinel must be present in map")
        } else {
            map.iter()
                .find(|(k, _)| k.func == "main")
                .expect("'main' function must be present in callgrind output")
        };

        self.fold(&map, key, value, "");
        Ok(())
    }
}
