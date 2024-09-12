use std::path::PathBuf;

use anyhow::Result;
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;

use super::error_metric_parser::ErrorMetricLogfileParser;
use super::{ToolOutputPath, ValgrindTool};
use crate::api::ErrorMetricKind;
use crate::runner::dhat::logfile_parser::DhatLogfileParser;
use crate::runner::summary::{CostsKind, CostsSummary, CostsSummaryType, ToolRunSummary};
use crate::util::EitherOrBoth;

// The different regex have to consider --time-stamp=yes
lazy_static! {
    pub static ref EXTRACT_FIELDS_RE: Regex = regex::Regex::new(
        r"^\s*(==|--)([0-9:.]+\s+)?[0-9]+(==|--)\s*(?<key>.*?)\s*:\s*(?<value>.*)\s*$"
    )
    .expect("Regex should compile");
    pub static ref EMPTY_LINE_RE: Regex =
        regex::Regex::new(r"^\s*(==|--)([0-9:.]+\s+)?[0-9]+(==|--)\s*$")
            .expect("Regex should compile");
    pub static ref STRIP_PREFIX_RE: Regex =
        regex::Regex::new(r"^\s*(==|--)([0-9:.]+\s+)?[0-9]+(==|--) (?<rest>.*)$")
            .expect("Regex should compile");
    static ref EXTRACT_PID_RE: Regex =
        regex::Regex::new(r"^\s*(==|--)([0-9:.]+\s+)?(?<pid>[0-9]+)(==|--).*")
            .expect("Regex should compile");
    static ref EXTRACT_ERRORS_RE: Regex =
        regex::Regex::new(r"^.*?(?<errors>[0-9]+).*$").expect("Regex should compile");
    static ref EXTRACT_ERROR_SUMMARY_RE: Regex = regex::Regex::new(
        r"^.*?(?<err>[0-9]+).*(<?<ctxs>[0-9]+).*(<?<s_err>[0-9]+).*(<?<s_ctxs>[0-9]+)$"
    )
    .expect("Regex should compile");
}

#[derive(Debug, Clone)]
pub struct LogfileSummary {
    pub command: PathBuf,
    pub pid: i32,
    pub parent_pid: Option<i32>,
    pub details: Vec<String>,
    pub log_path: PathBuf,
    pub costs: CostsKind,
}

/// TODO: Use the Parser trait instead if possible
pub trait LogfileParser {
    fn parse_single(&self, path: PathBuf) -> Result<LogfileSummary>;

    fn parse(&self, output_path: &ToolOutputPath) -> Result<Vec<LogfileSummary>> {
        let log_path = output_path.to_log_output();
        debug!("{}: Parsing log file '{}'", output_path.tool.id(), log_path);

        let mut summaries = vec![];
        let Ok(paths) = log_path.real_paths() else {
            return Ok(vec![]);
        };

        for path in paths {
            let summary = self.parse_single(path)?;
            summaries.push(summary);
        }

        summaries.sort_by_key(|x| x.pid);
        Ok(summaries)
    }

    fn merge_logfile_summaries(
        &self,
        old: Vec<LogfileSummary>,
        new: Vec<LogfileSummary>,
    ) -> Vec<ToolRunSummary>;

    fn parse_merge(
        &self,
        output_path: &ToolOutputPath,
        old: Vec<LogfileSummary>,
    ) -> Result<Vec<ToolRunSummary>> {
        let new = self.parse(output_path)?;
        Ok(self.merge_logfile_summaries(old, new))
    }
}

impl LogfileSummary {
    fn raw_into_tool_run(self) -> ToolRunSummary {
        ToolRunSummary {
            command: self.command.to_string_lossy().to_string(),
            old_pid: None,
            old_parent_pid: None,
            pid: None,
            parent_pid: None,
            details: (!self.details.is_empty()).then(|| self.details.join("\n")),
            log_path: self.log_path,
            costs_summary: CostsSummaryType::default(),
        }
    }

    pub fn old_into_tool_run(self) -> ToolRunSummary {
        let costs_summary = match self.costs {
            CostsKind::None => CostsSummaryType::None,
            CostsKind::DhatCosts(ref costs) => {
                CostsSummaryType::DhatSummary(CostsSummary::new(EitherOrBoth::Right(costs.clone())))
            }
            CostsKind::ErrorCosts(ref costs) => CostsSummaryType::ErrorSummary(CostsSummary::new(
                EitherOrBoth::Right(costs.clone()),
            )),
        };
        let old_pid = Some(self.pid);
        let old_parent_pid = self.parent_pid;
        ToolRunSummary {
            old_pid,
            old_parent_pid,
            costs_summary,
            ..self.raw_into_tool_run()
        }
    }

    pub fn new_into_tool_run(self) -> ToolRunSummary {
        let costs_summary = match self.costs {
            CostsKind::DhatCosts(ref costs) => {
                CostsSummaryType::DhatSummary(CostsSummary::new(EitherOrBoth::Left(costs.clone())))
            }
            CostsKind::ErrorCosts(ref costs) => {
                CostsSummaryType::ErrorSummary(CostsSummary::new(EitherOrBoth::Left(costs.clone())))
            }
            CostsKind::None => CostsSummaryType::None,
        };
        let pid = Some(self.pid);
        let parent_pid = self.parent_pid;
        ToolRunSummary {
            pid,
            parent_pid,
            costs_summary,
            ..self.raw_into_tool_run()
        }
    }

    pub fn merge(self, old: &LogfileSummary) -> ToolRunSummary {
        assert_eq!(self.command, old.command);
        let costs_summary = match (&self.costs, &old.costs) {
            (CostsKind::None, CostsKind::None) => CostsSummaryType::None,
            (CostsKind::DhatCosts(new), CostsKind::DhatCosts(old)) => {
                CostsSummaryType::DhatSummary(CostsSummary::new(EitherOrBoth::Both((
                    new.clone(),
                    old.clone(),
                ))))
            }
            (CostsKind::ErrorCosts(new), CostsKind::ErrorCosts(old)) => {
                CostsSummaryType::ErrorSummary(CostsSummary::new(EitherOrBoth::Both((
                    new.clone(),
                    old.clone(),
                ))))
            }
            (CostsKind::None, CostsKind::DhatCosts(old)) => {
                CostsSummaryType::DhatSummary(CostsSummary::new(EitherOrBoth::Right(old.clone())))
            }
            (CostsKind::None, CostsKind::ErrorCosts(old)) => {
                CostsSummaryType::ErrorSummary(CostsSummary::new(EitherOrBoth::Right(old.clone())))
            }
            (CostsKind::DhatCosts(new), CostsKind::None) => {
                CostsSummaryType::DhatSummary(CostsSummary::new(EitherOrBoth::Left(new.clone())))
            }
            (CostsKind::ErrorCosts(new), CostsKind::None) => {
                CostsSummaryType::ErrorSummary(CostsSummary::new(EitherOrBoth::Left(new.clone())))
            }
            _ => panic!("The logfile summaries of new and old costs should match"),
        };

        let old_pid = Some(old.pid);
        let old_parent_pid = old.parent_pid;
        let pid = Some(self.pid);
        let parent_pid = self.parent_pid;
        ToolRunSummary {
            old_pid,
            old_parent_pid,
            pid,
            parent_pid,
            costs_summary,
            ..self.raw_into_tool_run()
        }
    }
}

pub fn extract_pid(line: &str) -> i32 {
    EXTRACT_PID_RE
        .captures(line.trim())
        .expect("Log output should not be malformed")
        .name("pid")
        .expect("Log output should contain pid")
        .as_str()
        .parse::<i32>()
        .expect("Pid should be valid")
}

impl ValgrindTool {
    pub fn to_parser(self, root_dir: PathBuf) -> Box<dyn LogfileParser> {
        match self {
            ValgrindTool::DHAT => Box::new(DhatLogfileParser { root_dir }),
            _ => Box::new(ErrorMetricLogfileParser { root_dir }),
        }
    }
}

impl LogfileSummary {
    pub fn has_errors(&self) -> bool {
        match &self.costs {
            CostsKind::None | CostsKind::DhatCosts(_) => false,
            CostsKind::ErrorCosts(costs) => costs
                .cost_by_kind(&ErrorMetricKind::Errors)
                .map_or(false, |e| e > 0),
        }
    }
}
