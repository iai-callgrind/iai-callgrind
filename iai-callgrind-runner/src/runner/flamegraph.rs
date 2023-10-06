use std::fmt::{Display, Write};
use std::fs::File;
use std::io::{BufWriter, Cursor, Write as IoWrite};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use inferno::flamegraph::{Direction, Options};
use log::{trace, warn};

use super::callgrind::flamegraph_parser::FlamegraphParser;
use super::callgrind::model::{Costs, EventType};
use super::callgrind::parser::{Parser, Sentinel};
use super::callgrind::CallgrindOutput;
use crate::api;

impl From<api::Direction> for Direction {
    fn from(value: api::Direction) -> Self {
        match value {
            api::Direction::TopToBottom => Direction::Inverted,
            api::Direction::BottomToTop => Direction::Straight,
        }
    }
}

pub struct Config {
    pub title: String,
    pub subtitle: String,
    pub ignore_missing: bool,
    pub direction: Direction,
    pub differential: bool,
}

pub struct Flamegraph {
    pub config: Config,
    pub event_types: Vec<EventType>,
    pub stacks: Stacks,
}

impl Flamegraph {
    pub fn new(heading: String, stacks: Stacks, config: api::FlamegraphConfig) -> Self {
        let (title, subtitle) = match (config.title, config.subtitle) {
            (None, None) => {
                let split = heading.split_once(' ').unwrap();
                (split.0.to_owned(), split.1.to_owned())
            }
            (None, Some(s)) => (heading, s),
            (Some(t), None) => (t, heading),
            (Some(t), Some(s)) => (t, s),
        };

        Self {
            stacks,
            event_types: config
                .event_types
                .iter()
                .map(|e| EventType::from(*e))
                .collect(),
            config: Config {
                title,
                subtitle,
                ignore_missing: config.ignore_missing,
                direction: config.direction.into(),
                differential: config.differential,
            },
        }
    }

    pub fn create(
        &self,
        callgrind_output: &CallgrindOutput,
        sentinel: Option<&Sentinel>,
        project_root: &Path,
    ) -> Result<()> {
        if self.stacks.is_empty() {
            warn!("Unable to create a flamegraph: No stacks found");
            return Ok(());
        }

        let mut options = Options::default();
        options.title = self.config.title.clone();
        options.subtitle = Some(self.config.subtitle.clone());
        options.direction = self.config.direction;

        let old_output = callgrind_output.to_old_output();

        #[allow(clippy::if_then_some_else_none)]
        let old_stacks = if self.config.differential && old_output.exists() {
            Some(FlamegraphParser::new(sentinel, project_root).parse(&old_output)?)
        } else {
            None
        };

        for event_type in &self.event_types {
            options.count_name = event_type.to_string();

            let stacks_lines = match self.stacks.encode(event_type) {
                Ok(s) => s,
                Err(_) if self.config.ignore_missing => continue,
                Err(error) => return Err(error),
            };

            let output = Output::init(callgrind_output.as_path(), event_type)?;
            create_flamegraph(
                &output,
                &mut options,
                stacks_lines.iter().map(std::string::String::as_str),
            )?;

            if let Some(old_stacks) = old_stacks.as_ref() {
                let old_stacks_lines = match old_stacks.encode(event_type) {
                    Ok(s) => s,
                    Err(_) if self.config.ignore_missing => continue,
                    Err(error) => return Err(error),
                };

                let cursor = Cursor::new(stacks_lines.join("\n"));
                let old_cursor = Cursor::new(old_stacks_lines.join("\n"));
                let mut result = Cursor::new(vec![]);

                let differential_options = inferno::differential::Options {
                    normalize: true,
                    strip_hex: Default::default(),
                };

                inferno::differential::from_readers(
                    differential_options,
                    old_cursor,
                    cursor,
                    &mut result,
                )
                .context("Failed creating a differential flamegraph")?;

                create_flamegraph(
                    &output.to_diff_output(),
                    &mut options,
                    String::from_utf8_lossy(result.get_ref()).lines(),
                )?;
            }
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

    // Convert stacks to stacks string format for this `EventType`
    //
    // # Errors
    //
    // If the event type was not present in the stacks
    pub fn encode(&self, event_type: &EventType) -> Result<Vec<String>> {
        let mut stacks = vec![];
        for stack in self.iter() {
            let s = stack.to_string(event_type)?;
            stacks.push(s);
        }
        Ok(stacks)
    }
}

pub struct Output(PathBuf);

impl Output {
    pub fn init<T>(path: T, event_type: &EventType) -> Result<Self>
    where
        T: AsRef<Path>,
    {
        let path = path.as_ref().with_extension(format!("{event_type}.svg"));
        if path.exists() {
            let old_svg = path.with_extension("old.svg");
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

    pub fn exists(&self) -> bool {
        self.0.exists()
    }

    pub fn to_diff_output(&self) -> Self {
        Self(self.0.with_extension("diff.svg"))
    }
}

fn create_flamegraph<'stacks>(
    output: &Output,
    options: &mut Options<'_>,
    stacks: impl Iterator<Item = &'stacks str>,
) -> Result<()> {
    let mut writer = BufWriter::new(output.create()?);
    inferno::flamegraph::from_lines(options, stacks, &mut writer)
        .with_context(|| format!("Failed creating a flamegraph at '{}'", output.0.display()))?;

    writer
        .flush()
        .with_context(|| format!("Failed flushing content to '{}'", output.0.display()))
}
