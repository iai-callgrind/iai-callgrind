use std::fs::File;
use std::io::{BufWriter, Cursor, Write as IoWrite};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use inferno::flamegraph::{Direction, Options};
use log::warn;

use super::flamegraph_parser::FlamegraphParser;
use super::parser::{Parser, Sentinel};
use crate::api::{self, EventKind, FlamegraphKind};
use crate::runner::summary::FlamegraphSummary;
use crate::runner::tool::ToolOutputPath;

#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct Config {
    pub kind: FlamegraphKind,
    pub negate_differential: bool,
    pub normalize_differential: bool,
    pub event_kinds: Vec<EventKind>,
    pub direction: Direction,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub min_width: f64,
}

#[derive(Debug, Clone)]
pub struct Flamegraph {
    pub config: Config,
}

pub struct Output(PathBuf);

impl From<api::FlamegraphConfig> for Config {
    fn from(value: api::FlamegraphConfig) -> Self {
        Self {
            kind: value.kind.unwrap_or(FlamegraphKind::All),
            negate_differential: value.negate_differential.unwrap_or_default(),
            normalize_differential: value.normalize_differential.unwrap_or(false),
            event_kinds: value
                .event_kinds
                .unwrap_or_else(|| vec![EventKind::EstimatedCycles]),
            direction: value
                .direction
                .map_or_else(|| Direction::Inverted, std::convert::Into::into),
            title: value.title.clone(),
            subtitle: value.subtitle.clone(),
            min_width: value.min_width.unwrap_or(0.1f64),
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
    pub fn new(heading: String, mut config: Config) -> Self {
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

        Self { config }
    }

    pub fn create(
        &self,
        callgrind_output_path: &ToolOutputPath,
        sentinel: Option<&Sentinel>,
        project_root: &Path,
    ) -> Result<Vec<FlamegraphSummary>> {
        if self.config.kind == FlamegraphKind::None {
            return Ok(vec![]);
        }
        let summarize_events = [
            EventKind::L1hits,
            EventKind::LLhits,
            EventKind::RamHits,
            EventKind::TotalRW,
            EventKind::EstimatedCycles,
        ];
        let summarize = self
            .config
            .event_kinds
            .iter()
            .any(|e| summarize_events.contains(e));

        let parser = FlamegraphParser::new(sentinel, project_root);
        // We need this map in all remaining cases of `FlamegraphKinds`
        let mut map = parser.parse(callgrind_output_path)?;
        if map.is_empty() {
            warn!("Unable to create a flamegraph: No stacks found");
            return Ok(vec![]);
        }

        let mut options = Options::default();
        options.negate_differentials = self.config.negate_differential;
        options.direction = self.config.direction;
        options.title = self
            .config
            .title
            .as_ref()
            .expect("A title must be present at this point")
            .clone();
        options.subtitle = self.config.subtitle.clone();
        options.min_width = self.config.min_width;

        let old_path = callgrind_output_path.to_base_path();

        #[allow(clippy::if_then_some_else_none)]
        let mut old_map = if (self.config.kind == FlamegraphKind::Differential
            || self.config.kind == FlamegraphKind::All)
            && old_path.exists()
        {
            Some(parser.parse(&old_path)?)
        } else {
            None
        };

        if summarize {
            map.make_summary()?;
            if let Some(map) = old_map.as_mut() {
                map.make_summary()?;
            }
        }

        let mut flamegraph_summaries = vec![];
        for event_kind in &self.config.event_kinds {
            let mut flamegraph_summary = FlamegraphSummary::new(*event_kind);

            options.count_name = event_kind.to_string();
            let stacks_lines = map.to_stack_format(event_kind)?;

            let output = Output::init(callgrind_output_path.to_path(), event_kind)?;
            if self.config.kind == FlamegraphKind::Regular
                || self.config.kind == FlamegraphKind::All
            {
                create_flamegraph(
                    &output,
                    &mut options,
                    stacks_lines.iter().map(std::string::String::as_str),
                )?;
                flamegraph_summary.regular_path = Some(output.as_path().to_owned());
            }

            // Is Some if FlamegraphKind::Differential or FlamegraphKind::Both
            if let Some(old_map) = old_map.as_ref() {
                let old_stacks_lines = old_map.to_stack_format(event_kind)?;

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

                let diff_output = output.to_diff_output();
                create_flamegraph(
                    &diff_output,
                    &mut options,
                    String::from_utf8_lossy(result.get_ref()).lines(),
                )?;

                flamegraph_summary.old_path = Some(output.to_old_path().as_path().to_owned());
                flamegraph_summary.diff_path = Some(diff_output.as_path().to_owned());
            }

            flamegraph_summaries.push(flamegraph_summary);
        }

        Ok(flamegraph_summaries)
    }
}

impl Output {
    pub fn init<T>(path: T, event_kind: &EventKind) -> Result<Self>
    where
        T: AsRef<Path>,
    {
        let output = Self(path.as_ref().with_extension(format!("{event_kind}.svg")));
        if output.exists() {
            let old_svg = output.to_old_path();
            std::fs::rename(output.as_path(), old_svg.as_path()).with_context(|| {
                format!(
                    "Failed moving flamegraph file '{}' -> '{}'",
                    &output.as_path().display(),
                    &old_svg.as_path().display(),
                )
            })?;
        }

        Ok(output)
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

    pub fn to_old_path(&self) -> Self {
        Self(self.0.with_extension("old.svg"))
    }

    pub fn as_path(&self) -> &Path {
        self.0.as_path()
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
