use std::fmt::{Display, Write};
use std::fs::File;
use std::io::{BufWriter, Write as IoWrite};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use inferno::flamegraph::Options;
use log::{trace, warn};

use super::callgrind::model::{Costs, EventType};

pub struct Flamegraph {
    pub types: Vec<EventType>,
    pub title: String,
    pub stacks: Stacks,
}

impl Flamegraph {
    pub fn create<T>(&self, path: T) -> Result<()>
    where
        T: AsRef<Path>,
    {
        if self.stacks.is_empty() {
            warn!("Unable to create a flamegraph: No stacks found");
            return Ok(());
        }

        let path = path.as_ref();
        for event_type in &self.types {
            let mut options = Options::default();
            options.title = self.title.clone();
            options.count_name = event_type.to_string();

            let mut stacks = vec![];
            for stack in self.stacks.iter() {
                stacks.push(stack.to_string(event_type)?);
            }

            let output = Output::init(path, event_type)?;
            let mut writer = BufWriter::new(output.create()?);
            inferno::flamegraph::from_lines(
                &mut options,
                stacks.iter().map(std::string::String::as_str),
                &mut writer,
            )
            .with_context(|| format!("Failed creating a flamegraph at '{}'", path.display()))?;

            writer
                .flush()
                .with_context(|| format!("Failed flushing content to '{}'", path.display()))?;
        }

        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct StackEntry {
    is_inline: bool,
    value: String,
}

impl Display for StackEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_inline {
            f.write_fmt(format_args!("[{}]", self.value))
        } else {
            f.write_str(&self.value)
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Stack {
    pub entries: Vec<StackEntry>,
    pub costs: Costs,
}

impl Stack {
    pub fn new<T>(item: T, costs: Costs, is_inline: bool) -> Self
    where
        T: Into<String>,
    {
        Self {
            entries: vec![StackEntry {
                value: item.into(),
                is_inline,
            }],
            costs,
        }
    }

    pub fn add<T>(&mut self, item: T, costs: Costs, is_inline: bool)
    where
        T: Into<String>,
    {
        self.entries.push(StackEntry {
            value: item.into(),
            is_inline,
        });
        self.costs = costs;
    }

    pub fn contains<T>(&self, item: T, is_inline: bool) -> bool
    where
        T: AsRef<str>,
    {
        let item = item.as_ref();
        self.entries
            .iter()
            .rev()
            .any(|e| e.is_inline == is_inline && e.value == item)
    }

    pub fn event_types(&self) -> Vec<EventType> {
        self.costs.event_types()
    }

    pub fn to_string(&self, event_type: &EventType) -> Result<String> {
        let mut result = String::new();
        if let Some((first, suffix)) = self.entries.split_first() {
            write!(&mut result, "{first}").unwrap();
            for element in suffix {
                write!(&mut result, ";{element}").unwrap();
            }
            write!(
                &mut result,
                " {}",
                self.costs.cost_by_type(event_type).ok_or_else(|| anyhow!(
                    "Failed creating flamegraph stack: Missing event type '{event_type}'. \
                     Possible event types are: '{}'",
                    self.event_types().iter().fold(String::new(), |mut a, e| {
                        write!(a, ", {e}").unwrap();
                        a
                    })
                ))?
            )
            .unwrap();
        }

        Ok(result)
    }
}

#[derive(Debug, Default)]
pub struct Stacks(pub Vec<Stack>);

impl Stacks {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn push(&mut self, value: Stack) {
        self.0.push(value);
    }

    pub fn add<T>(&mut self, item: T, costs: Costs, is_inline: bool, base: Option<&Stack>)
    where
        T: Into<String>,
    {
        let stack = if let Some(last) = base {
            let mut stack = last.clone();
            stack.add(item, costs, is_inline);
            stack
        } else {
            Stack::new(item, costs, is_inline)
        };

        trace!("Pushing stack: {:?}", stack);
        self.push(stack);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Stack> {
        self.0.iter()
    }

    pub fn last(&self) -> Option<&Stack> {
        self.0.last()
    }
}

pub struct Output(pub PathBuf);

impl Output {
    pub fn init<T>(path: T, event_type: &EventType) -> Result<Self>
    where
        T: AsRef<Path>,
    {
        let path = path.as_ref().with_extension(format!("{event_type}.svg"));
        if path.exists() {
            let old_svg = path.with_extension("svg.old");
            std::fs::copy(&path, &old_svg).with_context(|| {
                format!(
                    "Failed copying flamegraph file '{}' -> '{}'",
                    &path.display(),
                    &old_svg.display(),
                )
            })?;
        }

        Ok(Self(path))
    }

    pub fn create(&self) -> Result<File> {
        File::create(&self.0)
            .with_context(|| format!("Failed creating flamegraph file '{}'", self.0.display()))
    }
}
