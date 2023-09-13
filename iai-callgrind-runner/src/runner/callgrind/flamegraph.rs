use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use inferno::flamegraph::Options;
use log::trace;

use super::hashmap_parser::{HashMapParser, Id};
use super::{CallgrindOutput, CallgrindParser, Sentinel};
use crate::error::Result;
use crate::runner::callgrind::hashmap_parser::Record;

#[derive(Debug)]
pub struct CallgrindFlamegraph {
    stacks: Vec<String>,
    project_root: PathBuf,
    sentinel: Option<Sentinel>,
    title: String,
}

impl CallgrindFlamegraph {
    pub fn new(title: String, sentinel: Option<&Sentinel>, project_root: &Path) -> Self {
        Self {
            title,
            sentinel: sentinel.cloned(),
            stacks: Vec::default(),
            project_root: project_root.to_owned(),
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
        // TODO: MAKE the choice of a title for the svg files configurable
        // TODO: MAKE the choice of a name for the counts configurable??
        let stack = format!("{this} {}", value.self_costs[0]);
        if self.stacks.last().map_or(false, |last| {
            last.rsplit_once(|c| c == ' ').unwrap().0 == this
        }) {
            let stack = format!("{last};spacer 0");
            trace!("Pushing spacer to stack: {}", &stack);
            self.stacks.push(stack);
        }
        trace!("Pushing stack: {}", &stack);
        self.stacks.push(stack);
        for inline in &value.inlines {
            if let Some(value) = inline.fi.as_ref().or(inline.fe.as_ref()) {
                let path = PathBuf::from(&value);
                let path = path.strip_prefix(&self.project_root).unwrap_or(&path);

                let stack = format!("{this};[{}] {}", path.display(), inline.costs[0]);
                trace!("Pushing stack: {}", &stack);
                self.stacks.push(stack);
            }
        }

        for cfn in &value.cfns {
            // if cfn.cfn.is_empty() {
            //     continue;
            // }

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
        let output_file = File::create(dest).unwrap();

        let mut options = Options::default();
        options.count_name = "Instructions".to_owned();
        options.title = self.title.clone();

        inferno::flamegraph::from_lines(
            &mut options,
            self.stacks.iter().map(std::string::String::as_str),
            output_file,
        )
        .map_err(|error| {
            crate::error::IaiCallgrindError::Other(format!("Error creating flamegraph: {error}"))
        })
    }
}

impl CallgrindParser for CallgrindFlamegraph {
    fn parse<T>(&mut self, file: T) -> Result<()>
    where
        T: AsRef<CallgrindOutput>,
    {
        let mut parser = HashMapParser {
            sentinel: self.sentinel.clone(),
            ..Default::default()
        };
        parser.parse(file)?;
        let map = parser.map;

        // Let's find our entry point which defaults to "main"
        let (key, value) = if let Some(key) = parser.resolved_sentinel {
            map.get_key_value(&key).unwrap()
        } else {
            map.iter()
                .find(|(k, _)| k.func == "main")
                .expect("Key with sentinel must be present in callgrind output")
        };

        self.fold(&map, key, value, "");
        Ok(())
    }
}
