use std::fs::File;
use std::io::{BufWriter, Cursor, Write as IoWrite};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use inferno::flamegraph::{Direction, Options};

use super::flamegraph_parser::{FlamegraphMap, FlamegraphParser};
use super::parser::Sentinel;
use crate::api::{self, EventKind, FlamegraphKind};
use crate::runner::summary::{BaselineKind, BaselineName, FlamegraphSummary};
use crate::runner::tool::{Parser, ToolOutputPath, ToolOutputPathKind};

#[derive(Debug)]
pub struct BaselineFlamegraphGenerator {
    pub baseline_kind: BaselineKind,
}

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

#[derive(Debug)]
pub struct LoadBaselineFlamegraphGenerator {
    pub loaded_baseline: BaselineName,
    pub baseline: BaselineName,
}

#[derive(Debug, Clone)]
struct OutputPath {
    pub kind: OutputPathKind,
    pub event_kind: EventKind,
    pub baseline_kind: BaselineKind,
    pub dir: PathBuf,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OutputPathKind {
    Regular,
    Old,
    Base(String),
    DiffOld,
    DiffBase(String),
    DiffBases(String, String),
}

#[derive(Debug)]
pub struct SaveBaselineFlamegraphGenerator {
    pub baseline: BaselineName,
}

pub trait FlamegraphGenerator {
    fn create(
        &self,
        flamegraph: &Flamegraph,
        tool_output_path: &ToolOutputPath,
        sentinel: Option<&Sentinel>,
        project_root: &Path,
    ) -> Result<Vec<FlamegraphSummary>>;
}

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

impl FlamegraphGenerator for BaselineFlamegraphGenerator {
    fn create(
        &self,
        flamegraph: &Flamegraph,
        tool_output_path: &ToolOutputPath,
        sentinel: Option<&Sentinel>,
        project_root: &Path,
    ) -> Result<Vec<FlamegraphSummary>> {
        // We need the dummy path just to cleanup and organize the output files independently from
        // the EventKind of the OutputPath
        let mut output_path = OutputPath::new(tool_output_path, EventKind::Ir);
        output_path.init()?;
        output_path.to_diff_path().clear(true)?;
        output_path.shift(true)?;

        if flamegraph.config.kind == FlamegraphKind::None
            || flamegraph.config.event_kinds.is_empty()
        {
            return Ok(vec![]);
        }

        let (map, base_map) = flamegraph.parse(tool_output_path, sentinel, project_root, false)?;

        let mut flamegraph_summaries = vec![];
        for event_kind in &flamegraph.config.event_kinds {
            let mut flamegraph_summary = FlamegraphSummary::new(*event_kind);
            output_path.set_event_kind(*event_kind);

            let stacks_lines = map.to_stack_format(event_kind)?;

            if flamegraph.is_regular() {
                Flamegraph::write(
                    &output_path,
                    &mut flamegraph.options(*event_kind, output_path.file_name()),
                    stacks_lines.iter().map(std::string::String::as_str),
                )?;
                flamegraph_summary.regular_path = Some(output_path.to_path());
            }

            // Is Some if FlamegraphKind::Differential or FlamegraphKind::All
            if let Some(base_map) = base_map.as_ref() {
                Flamegraph::create_differential(
                    &output_path,
                    &mut flamegraph.options(*event_kind, output_path.to_diff_path().file_name()),
                    base_map,
                    // This unwrap is safe since we always have differential options if the
                    // flamegraph kind is differential
                    flamegraph.differential_options().unwrap(),
                    *event_kind,
                    &stacks_lines,
                )?;

                flamegraph_summary.base_path = Some(output_path.to_base_path().to_path());
                flamegraph_summary.diff_path = Some(output_path.to_diff_path().to_path());
            }

            flamegraph_summaries.push(flamegraph_summary);
        }

        Ok(flamegraph_summaries)
    }
}

impl Flamegraph {
    pub fn new(heading: String, mut config: Config) -> Self {
        if config.title.is_none() {
            config.title = Some(heading);
        }

        Self { config }
    }

    pub fn is_differential(&self) -> bool {
        matches!(
            self.config.kind,
            FlamegraphKind::Differential | FlamegraphKind::All
        )
    }

    pub fn is_regular(&self) -> bool {
        matches!(
            self.config.kind,
            FlamegraphKind::Regular | FlamegraphKind::All
        )
    }

    pub fn options(&self, event_kind: EventKind, subtitle: String) -> Options {
        let mut options = Options::default();
        options.negate_differentials = self.config.negate_differential;
        options.direction = self.config.direction;
        options.title = self
            .config
            .title
            .as_ref()
            .expect("A title must be present at this point")
            .clone();

        options.subtitle = if let Some(subtitle) = &self.config.subtitle {
            Some(subtitle.clone())
        } else {
            Some(subtitle)
        };

        options.min_width = self.config.min_width;
        options.count_name = event_kind.to_string();
        options
    }

    pub fn differential_options(&self) -> Option<inferno::differential::Options> {
        self.is_differential()
            .then(|| inferno::differential::Options {
                normalize: self.config.normalize_differential,
                ..Default::default()
            })
    }

    pub fn parse<P>(
        &self,
        tool_output_path: &ToolOutputPath,
        sentinel: Option<&Sentinel>,
        project_root: P,
        no_differential: bool,
    ) -> Result<(FlamegraphMap, Option<FlamegraphMap>)>
    where
        P: Into<PathBuf>,
    {
        let parser = FlamegraphParser::new(sentinel, project_root);
        // We need this map in all remaining cases of `FlamegraphKinds`
        let mut map = parser.parse(tool_output_path)?;
        if map.is_empty() {
            return Err(anyhow!("Unable to create a flamegraph: No stacks found"));
        }

        let base_path = tool_output_path.to_base_path();
        #[allow(clippy::if_then_some_else_none)]
        let mut base_map = if !no_differential && self.is_differential() && base_path.exists() {
            Some(parser.parse(&base_path)?)
        } else {
            None
        };

        if self.config.event_kinds.iter().any(EventKind::is_derived) {
            map.make_summary()?;
            if let Some(map) = base_map.as_mut() {
                map.make_summary()?;
            }
        }

        Ok((map, base_map))
    }

    fn create_differential(
        output_path: &OutputPath,
        options: &mut inferno::flamegraph::Options,
        base_map: &FlamegraphMap,
        differential_options: inferno::differential::Options,
        event_kind: EventKind,
        stacks_lines: &[String],
    ) -> Result<()> {
        let base_stacks_lines = base_map.to_stack_format(&event_kind)?;

        let cursor = Cursor::new(stacks_lines.join("\n"));
        let base_cursor = Cursor::new(base_stacks_lines.join("\n"));
        let mut result = Cursor::new(vec![]);

        inferno::differential::from_readers(differential_options, base_cursor, cursor, &mut result)
            .context("Failed creating a differential flamegraph")?;

        let diff_output_path = output_path.to_diff_path();
        Flamegraph::write(
            &diff_output_path,
            options,
            String::from_utf8_lossy(result.get_ref()).lines(),
        )
    }

    fn write<'stacks>(
        output_path: &OutputPath,
        options: &mut Options<'_>,
        stacks: impl Iterator<Item = &'stacks str>,
    ) -> Result<()> {
        let path = output_path.to_path();
        let mut writer = BufWriter::new(output_path.create()?);
        inferno::flamegraph::from_lines(options, stacks, &mut writer)
            .with_context(|| format!("Failed creating a flamegraph at '{}'", path.display()))?;

        writer
            .flush()
            .with_context(|| format!("Failed flushing content to '{}'", path.display()))
    }
}

impl FlamegraphGenerator for LoadBaselineFlamegraphGenerator {
    fn create(
        &self,
        flamegraph: &Flamegraph,
        tool_output_path: &ToolOutputPath,
        sentinel: Option<&Sentinel>,
        project_root: &Path,
    ) -> Result<Vec<FlamegraphSummary>> {
        // We need the dummy path just to cleanup and organize the output files independently from
        // the EventKind of the OutputPath
        let mut output_path = OutputPath::new(tool_output_path, EventKind::Ir);
        output_path.to_diff_path().clear(true)?;

        if flamegraph.config.kind == FlamegraphKind::None
            || flamegraph.config.event_kinds.is_empty()
            || !flamegraph.is_differential()
        {
            return Ok(vec![]);
        }

        let (map, base_map) = flamegraph
            .parse(tool_output_path, sentinel, project_root, false)
            .map(|(m, b)| (m, b.unwrap()))?;

        let mut flamegraph_summaries = vec![];
        for event_kind in &flamegraph.config.event_kinds {
            let mut flamegraph_summary = FlamegraphSummary::new(*event_kind);
            output_path.set_event_kind(*event_kind);

            Flamegraph::create_differential(
                &output_path,
                &mut flamegraph.options(*event_kind, output_path.to_diff_path().file_name()),
                &base_map,
                // This unwrap is safe since we always produce a differential flamegraph
                flamegraph.differential_options().unwrap(),
                *event_kind,
                &map.to_stack_format(event_kind)?,
            )?;

            flamegraph_summary.regular_path = Some(output_path.to_path());
            flamegraph_summary.base_path = Some(output_path.to_base_path().to_path());
            flamegraph_summary.diff_path = Some(output_path.to_diff_path().to_path());

            flamegraph_summaries.push(flamegraph_summary);
        }

        Ok(flamegraph_summaries)
    }
}

impl OutputPath {
    pub fn new(tool_output_path: &ToolOutputPath, event_kind: EventKind) -> Self {
        Self {
            kind: match &tool_output_path.kind {
                ToolOutputPathKind::Out | ToolOutputPathKind::Log => OutputPathKind::Regular,
                ToolOutputPathKind::OldOut | ToolOutputPathKind::OldLog => OutputPathKind::Old,
                ToolOutputPathKind::BaseLog(name) | ToolOutputPathKind::Base(name) => {
                    OutputPathKind::Base(name.clone())
                }
            },
            event_kind,
            baseline_kind: tool_output_path.baseline_kind.clone(),
            dir: tool_output_path.dir.clone(),
            name: tool_output_path.name.clone(),
        }
    }

    pub fn init(&self) -> Result<()> {
        std::fs::create_dir_all(&self.dir)
            .with_context(|| {
                format!(
                    "Failed creating flamegraph directory '{}'",
                    self.dir.display()
                )
            })
            .map_err(Into::into)
    }

    pub fn create(&self) -> Result<File> {
        let path = self.to_path();
        File::create(&path)
            .with_context(|| format!("Failed creating flamegraph file '{}'", path.display()))
    }

    pub fn clear(&self, ignore_event_kind: bool) -> Result<()> {
        for path in self.real_paths(ignore_event_kind)? {
            std::fs::remove_file(path)?;
        }

        Ok(())
    }

    pub fn clear_diff(&self) -> Result<()> {
        let extension = match &self.baseline_kind {
            BaselineKind::Old => "diff.old.svg".to_owned(),
            BaselineKind::Name(name) => format!("diff.base@{name}.svg"),
        };
        for entry in std::fs::read_dir(&self.dir)
            .with_context(|| format!("Failed reading directory '{}'", self.dir.display()))?
        {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            if let Some(suffix) =
                file_name.strip_prefix(format!("callgrind.{}.flamegraph.", &self.name).as_str())
            {
                let path = entry.path();
                if suffix.ends_with(extension.as_str()) {
                    std::fs::remove_file(&path).with_context(|| {
                        format!("Failed removing flamegraph file: '{}'", path.display())
                    })?;
                }
                if let BaselineKind::Name(name) = &self.baseline_kind {
                    if suffix.split_once('.').map_or(false, |(_, s)| {
                        s.starts_with(format!("base@{name}.diff.").as_str())
                    }) {
                        std::fs::remove_file(&path).with_context(|| {
                            format!("Failed removing flamegraph file: '{}'", path.display())
                        })?;
                    }
                } else {
                    // do nothing
                }
            }
        }

        Ok(())
    }

    pub fn shift(&self, ignore_event_kind: bool) -> Result<()> {
        match &self.baseline_kind {
            BaselineKind::Old => {
                self.to_base_path().clear(ignore_event_kind)?;
                for path in self.real_paths(ignore_event_kind)? {
                    let new_path = path.with_extension("old.svg");
                    std::fs::rename(&path, &new_path).with_context(|| {
                        format!(
                            "Failed moving flamegraph file from '{}' to '{}'",
                            path.display(),
                            new_path.display()
                        )
                    })?;
                }
                Ok(())
            }
            BaselineKind::Name(_) => self.clear(ignore_event_kind),
        }
    }

    pub fn to_diff_path(&self) -> Self {
        Self {
            kind: match (&self.kind, &self.baseline_kind) {
                (OutputPathKind::Regular, BaselineKind::Old) => OutputPathKind::DiffOld,
                (OutputPathKind::Regular, BaselineKind::Name(name)) => {
                    OutputPathKind::DiffBase(name.to_string())
                }
                (OutputPathKind::Base(name), BaselineKind::Name(other)) => {
                    OutputPathKind::DiffBases(name.to_string(), other.to_string())
                }
                // TODO: NOT UNREACHABLE
                (OutputPathKind::Old | OutputPathKind::Base(_), _) => unreachable!(),
                (value, _) => value.clone(),
            },
            ..self.clone()
        }
    }

    pub fn to_base_path(&self) -> Self {
        Self {
            kind: match &self.baseline_kind {
                BaselineKind::Old => OutputPathKind::Old,
                BaselineKind::Name(name) => OutputPathKind::Base(name.to_string()),
            },
            ..self.clone()
        }
    }

    pub fn extension(&self) -> String {
        match &self.kind {
            OutputPathKind::Regular => format!("flamegraph.{}.svg", self.event_kind.to_name()),
            OutputPathKind::Old => format!("flamegraph.{}.old.svg", self.event_kind.to_name()),
            OutputPathKind::Base(name) => {
                format!("flamegraph.{}.base@{name}.svg", self.event_kind.to_name())
            }
            OutputPathKind::DiffOld => {
                format!("flamegraph.{}.diff.old.svg", self.event_kind.to_name())
            }
            OutputPathKind::DiffBase(name) => {
                format!(
                    "flamegraph.{}.diff.base@{name}.svg",
                    self.event_kind.to_name()
                )
            }
            OutputPathKind::DiffBases(name, base) => {
                format!(
                    "flamegraph.{}.base@{name}.diff.base@{base}.svg",
                    self.event_kind.to_name()
                )
            }
        }
    }

    pub fn set_event_kind(&mut self, event_kind: EventKind) {
        self.event_kind = event_kind;
    }

    pub fn real_paths(&self, ignore_event_kind: bool) -> Result<Vec<PathBuf>> {
        let mut paths = vec![];
        let extension = self.extension();
        let to_match = if ignore_event_kind {
            extension.splitn(3, '.').last().unwrap()
        } else {
            extension.strip_prefix("flamegraph.").unwrap()
        };
        for entry in std::fs::read_dir(&self.dir)
            .with_context(|| format!("Failed reading directory '{}'", self.dir.display()))?
        {
            let path = entry?;
            let file_name = path.file_name().to_string_lossy().to_string();
            if let Some(suffix) =
                file_name.strip_prefix(format!("callgrind.{}.flamegraph.", &self.name).as_str())
            {
                let is_match = if ignore_event_kind {
                    suffix
                        .split_once('.')
                        .map_or(false, |(_event_kind, rest)| rest == to_match)
                } else {
                    suffix == to_match
                };
                if is_match {
                    paths.push(path.path());
                }
            }
        }
        Ok(paths)
    }

    pub fn file_name(&self) -> String {
        format!("callgrind.{}.{}", self.name, self.extension())
    }

    pub fn to_path(&self) -> PathBuf {
        self.dir.join(self.file_name())
    }
}

impl FlamegraphGenerator for SaveBaselineFlamegraphGenerator {
    fn create(
        &self,
        flamegraph: &Flamegraph,
        tool_output_path: &ToolOutputPath,
        sentinel: Option<&Sentinel>,
        project_root: &Path,
    ) -> Result<Vec<FlamegraphSummary>> {
        // We need the dummy path just to cleanup and organize the output files independently from
        // the EventKind of the OutputPath
        let mut output_path = OutputPath::new(tool_output_path, EventKind::Ir);
        output_path.init()?;
        output_path.clear(true)?;
        output_path.clear_diff()?;

        if flamegraph.config.kind == FlamegraphKind::None
            || flamegraph.config.event_kinds.is_empty()
            || !flamegraph.is_regular()
        {
            return Ok(vec![]);
        }

        let (map, _) = flamegraph.parse(tool_output_path, sentinel, project_root, true)?;

        let mut flamegraph_summaries = vec![];
        for event_kind in &flamegraph.config.event_kinds {
            let mut flamegraph_summary = FlamegraphSummary::new(*event_kind);
            output_path.set_event_kind(*event_kind);

            Flamegraph::write(
                &output_path,
                &mut flamegraph.options(*event_kind, output_path.file_name()),
                map.to_stack_format(event_kind)?.iter().map(String::as_str),
            )?;

            flamegraph_summary.regular_path = Some(output_path.to_path());
            flamegraph_summaries.push(flamegraph_summary);
        }

        Ok(flamegraph_summaries)
    }
}
