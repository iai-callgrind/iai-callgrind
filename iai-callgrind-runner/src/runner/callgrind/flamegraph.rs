use std::fs::File;
use std::io::{BufWriter, Cursor, Write as IoWrite};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use inferno::flamegraph::{Direction, Options};
use log::warn;

use super::flamegraph_parser::{FlamegraphMap, FlamegraphParser};
use super::model::EventType;
use super::parser::{Parser, Sentinel};
use super::CallgrindOutput;
use crate::api;

#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct Config {
    pub enable: bool,
    pub enable_regular: bool,
    pub enable_differential: bool,
    pub negate_differential: bool,
    pub normalize_differential: bool,
    pub event_types: Vec<EventType>,
    pub ignore_missing: bool,
    pub direction: Direction,
    pub flamechart: bool,
    pub title: Option<String>,
    pub subtitle: Option<String>,
}

pub struct Flamegraph {
    pub config: Config,
    pub map: FlamegraphMap,
}

pub struct Output(PathBuf);

impl From<api::FlamegraphConfig> for Config {
    fn from(value: api::FlamegraphConfig) -> Self {
        Self {
            enable: value.enable.unwrap_or(true),
            enable_regular: value.enable_regular.unwrap_or(true),
            enable_differential: value.enable_differential.unwrap_or(true),
            negate_differential: value.negate_differential.unwrap_or_default(),
            normalize_differential: value.normalize_differential.unwrap_or(false),
            event_types: value.event_types.map_or_else(
                || vec![EventType::Ir],
                |events| events.iter().map(|e| EventType::from(*e)).collect(),
            ),
            ignore_missing: value.ignore_missing.unwrap_or(false),
            direction: value
                .direction
                .map_or_else(Direction::default, std::convert::Into::into),
            flamechart: value.flamechart.unwrap_or(false),
            title: value.title.clone(),
            subtitle: value.subtitle.clone(),
        }
    }
}

impl From<api::Direction> for Direction {
    fn from(value: api::Direction) -> Self {
        match value {
            api::Direction::TopToBottom => Direction::Inverted,
            api::Direction::BottomToTop => Direction::Straight,
        }
    }
}

impl Flamegraph {
    pub fn new(heading: String, map: FlamegraphMap, mut config: Config) -> Self {
        let (title, subtitle) = match (config.title, config.subtitle) {
            (None, None) => heading.split_once(' ').map_or_else(
                || (heading.clone(), None),
                |(k, v)| (k.to_owned(), Some(v.to_owned())),
            ),
            (None, Some(s)) => (heading, Some(s)),
            (Some(t), None) => (t, Some(heading)),
            (Some(t), Some(s)) => (t, Some(s)),
        };

        config.title = Some(title);
        config.subtitle = subtitle;

        Self { config, map }
    }

    pub fn create(
        &self,
        callgrind_output: &CallgrindOutput,
        sentinel: Option<&Sentinel>,
        project_root: &Path,
    ) -> Result<()> {
        if self.map.is_empty() {
            warn!("Unable to create a flamegraph: No stacks found");
            return Ok(());
        }

        let mut options = Options::default();
        options.negate_differentials = self.config.negate_differential;
        options.direction = self.config.direction;
        options.flame_chart = self.config.flamechart;
        options.title = self
            .config
            .title
            .as_ref()
            .expect("A title must be present at this point")
            .clone();
        options.subtitle = self.config.subtitle.clone();

        let old_output = callgrind_output.to_old_output();

        #[allow(clippy::if_then_some_else_none)]
        let old_map = if self.config.enable_differential && old_output.exists() {
            Some(FlamegraphParser::new(sentinel, project_root).parse(&old_output)?)
        } else {
            None
        };

        for event_type in &self.config.event_types {
            options.count_name = event_type.to_string();

            let stacks_lines = match self.map.to_stack_format(event_type) {
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

            if let Some(old_map) = old_map.as_ref() {
                let old_stacks_lines = match old_map.to_stack_format(event_type) {
                    Ok(s) => s,
                    Err(_) if self.config.ignore_missing => continue,
                    Err(error) => return Err(error),
                };

                let cursor = Cursor::new(stacks_lines.join("\n"));
                let old_cursor = Cursor::new(old_stacks_lines.join("\n"));
                let mut result = Cursor::new(vec![]);

                let differential_options = inferno::differential::Options {
                    normalize: self.config.normalize_differential,
                    ..Default::default()
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
