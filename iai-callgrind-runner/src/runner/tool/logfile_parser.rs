use std::path::PathBuf;

use anyhow::Result;
use itertools::Itertools;
use lazy_static::lazy_static;
use log::debug;
use regex::Regex;

use super::error_metric_parser::ErrorMetricLogfileParser;
use super::generic_parser::GenericLogfileParser;
use super::{ToolOutputPath, ValgrindTool};
use crate::runner::costs::Costs;
use crate::runner::dhat::logfile_parser::DhatLogfileParser;
use crate::runner::summary::{
    CostsKind, CostsSummary, CostsSummaryType, ToolRunInfo, ToolRunSummaries, ToolRunSummary,
};
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

#[derive(Debug)]
pub struct Header {
    pub command: PathBuf,
    pub pid: i32,
    pub parent_pid: Option<i32>,
}

#[derive(Debug)]
pub struct Logfile {
    pub path: PathBuf,
    pub header: Header,
    pub details: Vec<String>,
    pub costs: CostsKind,
}

impl From<Logfile> for ToolRunInfo {
    fn from(value: Logfile) -> Self {
        Self {
            command: value.header.command.display().to_string(),
            pid: value.header.pid,
            parent_pid: value.header.parent_pid,
            details: (!value.details.is_empty()).then(|| value.details.join("\n")),
            path: value.path,
        }
    }
}

#[derive(Debug)]
pub struct LogfileSummary {
    pub logfile: EitherOrBoth<Logfile>,
    pub costs_summary: CostsSummaryType,
}

#[derive(Debug)]
pub struct LogfileSummaries {
    data: Vec<LogfileSummary>,
    total: CostsSummaryType,
}

// TODO: REFACTOR THIS
// TODO: IMPLEMENT AND SORT INTO IMPL SECTION
// Logfiles are separated per process but not per threads by any tool
impl LogfileSummaries {
    pub fn new(logfiles: EitherOrBoth<Vec<Logfile>>) -> Self {
        let mut total = None;
        let summaries: Vec<LogfileSummary> = match logfiles {
            EitherOrBoth::Left(new) => new
                .into_iter()
                .map(|logfile| {
                    let costs_summary = match &logfile.costs {
                        CostsKind::None => {
                            total.get_or_insert(CostsSummaryType::None);
                            CostsSummaryType::None
                        }
                        CostsKind::DhatCosts(costs) => {
                            let summary = CostsSummaryType::DhatSummary(CostsSummary::new(
                                EitherOrBoth::Left(costs.clone()),
                            ));
                            let total_mut = total.get_or_insert(CostsSummaryType::DhatSummary(
                                CostsSummary::new(EitherOrBoth::Left(Costs::empty())),
                            ));
                            total_mut.add_mut(&summary);
                            summary
                        }
                        CostsKind::ErrorCosts(costs) => {
                            let summary = CostsSummaryType::ErrorSummary(CostsSummary::new(
                                EitherOrBoth::Left(costs.clone()),
                            ));
                            let total_mut = total.get_or_insert(CostsSummaryType::ErrorSummary(
                                CostsSummary::new(EitherOrBoth::Left(Costs::empty())),
                            ));
                            total_mut.add_mut(&summary);
                            summary
                        }
                        CostsKind::CallgrindCosts(costs) => {
                            let summary = CostsSummaryType::CallgrindSummary(CostsSummary::new(
                                EitherOrBoth::Left(costs.clone()),
                            ));
                            let total_mut =
                                total.get_or_insert(CostsSummaryType::CallgrindSummary(
                                    CostsSummary::new(EitherOrBoth::Left(Costs::empty())),
                                ));
                            total_mut.add_mut(&summary);
                            summary
                        }
                    };

                    LogfileSummary {
                        logfile: EitherOrBoth::Left(logfile),
                        costs_summary,
                    }
                })
                .collect(),
            EitherOrBoth::Right(old) => old
                .into_iter()
                .map(|logfile| {
                    let costs_summary = match &logfile.costs {
                        CostsKind::None => {
                            total.get_or_insert(CostsSummaryType::None);
                            CostsSummaryType::None
                        }
                        CostsKind::DhatCosts(costs) => {
                            let summary = CostsSummaryType::DhatSummary(CostsSummary::new(
                                EitherOrBoth::Right(costs.clone()),
                            ));
                            let total_mut = total.get_or_insert(CostsSummaryType::DhatSummary(
                                CostsSummary::new(EitherOrBoth::Right(Costs::empty())),
                            ));
                            total_mut.add_mut(&summary);
                            summary
                        }
                        CostsKind::ErrorCosts(costs) => {
                            let summary = CostsSummaryType::ErrorSummary(CostsSummary::new(
                                EitherOrBoth::Right(costs.clone()),
                            ));
                            let total_mut = total.get_or_insert(CostsSummaryType::ErrorSummary(
                                CostsSummary::new(EitherOrBoth::Right(Costs::empty())),
                            ));
                            total_mut.add_mut(&summary);
                            summary
                        }
                        CostsKind::CallgrindCosts(costs) => {
                            let summary = CostsSummaryType::CallgrindSummary(CostsSummary::new(
                                EitherOrBoth::Right(costs.clone()),
                            ));
                            let total_mut =
                                total.get_or_insert(CostsSummaryType::CallgrindSummary(
                                    CostsSummary::new(EitherOrBoth::Right(Costs::empty())),
                                ));
                            total_mut.add_mut(&summary);
                            summary
                        }
                    };

                    LogfileSummary {
                        logfile: EitherOrBoth::Right(logfile),
                        costs_summary,
                    }
                })
                .collect(),
            EitherOrBoth::Both(new, old) => new
                .into_iter()
                .zip_longest(old)
                .map(|either_or_both| match either_or_both {
                    itertools::EitherOrBoth::Both(new, old) => match (&new.costs, &old.costs) {
                        (CostsKind::None, CostsKind::None) => {
                            total.get_or_insert(CostsSummaryType::None);
                            LogfileSummary {
                                logfile: EitherOrBoth::Both(new, old),
                                costs_summary: CostsSummaryType::None,
                            }
                        }
                        (CostsKind::DhatCosts(new_costs), CostsKind::DhatCosts(old_costs)) => {
                            let costs_summary = CostsSummaryType::DhatSummary(CostsSummary::new(
                                EitherOrBoth::Both(new_costs.clone(), old_costs.clone()),
                            ));
                            let total_mut = total.get_or_insert(CostsSummaryType::DhatSummary(
                                CostsSummary::new(EitherOrBoth::Both(
                                    Costs::empty(),
                                    Costs::empty(),
                                )),
                            ));
                            total_mut.add_mut(&costs_summary);
                            LogfileSummary {
                                costs_summary,
                                logfile: EitherOrBoth::Both(new, old),
                            }
                        }
                        (CostsKind::ErrorCosts(new_costs), CostsKind::ErrorCosts(old_costs)) => {
                            let costs_summary = CostsSummaryType::ErrorSummary(CostsSummary::new(
                                EitherOrBoth::Both(new_costs.clone(), old_costs.clone()),
                            ));
                            let total_mut = total.get_or_insert(CostsSummaryType::ErrorSummary(
                                CostsSummary::new(EitherOrBoth::Both(
                                    Costs::empty(),
                                    Costs::empty(),
                                )),
                            ));
                            total_mut.add_mut(&costs_summary);
                            LogfileSummary {
                                costs_summary,
                                logfile: EitherOrBoth::Both(new, old),
                            }
                        }
                        (
                            CostsKind::CallgrindCosts(new_costs),
                            CostsKind::CallgrindCosts(old_costs),
                        ) => {
                            let costs_summary =
                                CostsSummaryType::CallgrindSummary(CostsSummary::new(
                                    EitherOrBoth::Both(new_costs.clone(), old_costs.clone()),
                                ));
                            let total_mut = total.get_or_insert(
                                CostsSummaryType::CallgrindSummary(CostsSummary::new(
                                    EitherOrBoth::Both(Costs::empty(), Costs::empty()),
                                )),
                            );
                            total_mut.add_mut(&costs_summary);
                            LogfileSummary {
                                costs_summary,
                                logfile: EitherOrBoth::Both(new, old),
                            }
                        }
                        _ => panic!("Cannot summarize incompatible log files"),
                    },
                    itertools::EitherOrBoth::Left(new) => match &new.costs {
                        CostsKind::None => {
                            total.get_or_insert(CostsSummaryType::None);
                            LogfileSummary {
                                costs_summary: CostsSummaryType::None,
                                logfile: EitherOrBoth::Left(new),
                            }
                        }
                        CostsKind::DhatCosts(new_costs) => {
                            let costs_summary = CostsSummaryType::DhatSummary(CostsSummary::new(
                                EitherOrBoth::Left(new_costs.clone()),
                            ));
                            let total_mut = total.get_or_insert(CostsSummaryType::DhatSummary(
                                CostsSummary::new(EitherOrBoth::Left(Costs::empty())),
                            ));
                            total_mut.add_mut(&costs_summary);
                            LogfileSummary {
                                costs_summary,
                                logfile: EitherOrBoth::Left(new),
                            }
                        }
                        CostsKind::ErrorCosts(new_costs) => {
                            let costs_summary = CostsSummaryType::ErrorSummary(CostsSummary::new(
                                EitherOrBoth::Left(new_costs.clone()),
                            ));
                            let total_mut = total.get_or_insert(CostsSummaryType::ErrorSummary(
                                CostsSummary::new(EitherOrBoth::Left(Costs::empty())),
                            ));
                            total_mut.add_mut(&costs_summary);
                            LogfileSummary {
                                costs_summary,
                                logfile: EitherOrBoth::Left(new),
                            }
                        }
                        CostsKind::CallgrindCosts(new_costs) => {
                            let costs_summary = CostsSummaryType::CallgrindSummary(
                                CostsSummary::new(EitherOrBoth::Left(new_costs.clone())),
                            );
                            let total_mut =
                                total.get_or_insert(CostsSummaryType::CallgrindSummary(
                                    CostsSummary::new(EitherOrBoth::Left(Costs::empty())),
                                ));
                            total_mut.add_mut(&costs_summary);
                            LogfileSummary {
                                costs_summary,
                                logfile: EitherOrBoth::Left(new),
                            }
                        }
                    },
                    itertools::EitherOrBoth::Right(old) => match &old.costs {
                        CostsKind::None => {
                            total.get_or_insert(CostsSummaryType::None);
                            LogfileSummary {
                                costs_summary: CostsSummaryType::None,
                                logfile: EitherOrBoth::Right(old),
                            }
                        }
                        CostsKind::DhatCosts(old_costs) => {
                            let costs_summary = CostsSummaryType::DhatSummary(CostsSummary::new(
                                EitherOrBoth::Right(old_costs.clone()),
                            ));

                            let total_mut = total.get_or_insert(CostsSummaryType::DhatSummary(
                                CostsSummary::new(EitherOrBoth::Right(Costs::empty())),
                            ));
                            total_mut.add_mut(&costs_summary);
                            LogfileSummary {
                                costs_summary,
                                logfile: EitherOrBoth::Right(old),
                            }
                        }
                        CostsKind::ErrorCosts(old_costs) => {
                            let costs_summary = CostsSummaryType::ErrorSummary(CostsSummary::new(
                                EitherOrBoth::Right(old_costs.clone()),
                            ));
                            let total_mut = total.get_or_insert(CostsSummaryType::ErrorSummary(
                                CostsSummary::new(EitherOrBoth::Right(Costs::empty())),
                            ));
                            total_mut.add_mut(&costs_summary);
                            LogfileSummary {
                                costs_summary,
                                logfile: EitherOrBoth::Right(old),
                            }
                        }
                        CostsKind::CallgrindCosts(old_costs) => {
                            let costs_summary = CostsSummaryType::CallgrindSummary(
                                CostsSummary::new(EitherOrBoth::Right(old_costs.clone())),
                            );
                            let total_mut =
                                total.get_or_insert(CostsSummaryType::CallgrindSummary(
                                    CostsSummary::new(EitherOrBoth::Right(Costs::empty())),
                                ));
                            total_mut.add_mut(&costs_summary);
                            LogfileSummary {
                                costs_summary,
                                logfile: EitherOrBoth::Right(old),
                            }
                        }
                    },
                })
                .collect(),
        };

        Self {
            data: summaries,
            total: total.expect("A total should be present"),
        }
    }

    pub fn into_tool_run_summaries(self) -> ToolRunSummaries {
        let summaries = self
            .data
            .into_iter()
            .map(|logfile_summary| match logfile_summary.logfile {
                EitherOrBoth::Left(new_logfile) => ToolRunSummary {
                    info: EitherOrBoth::Left(new_logfile.into()),
                    costs_summary: logfile_summary.costs_summary,
                },
                EitherOrBoth::Right(old_logfile) => ToolRunSummary {
                    info: EitherOrBoth::Right(old_logfile.into()),
                    costs_summary: logfile_summary.costs_summary,
                },
                EitherOrBoth::Both(new_logfile, old_logfile) => ToolRunSummary {
                    info: EitherOrBoth::Both(new_logfile.into(), old_logfile.into()),
                    costs_summary: logfile_summary.costs_summary,
                },
            })
            .collect();

        ToolRunSummaries {
            data: summaries,
            total: self.total,
        }
    }
}

pub trait LogfileParser {
    fn parse_single(&self, path: PathBuf) -> Result<Logfile>;
    fn parse(&self, output_path: &ToolOutputPath) -> Result<Vec<Logfile>> {
        let log_path = output_path.to_log_output();
        debug!("{}: Parsing log file '{}'", output_path.tool.id(), log_path);

        let mut logfiles = vec![];
        let Ok(paths) = log_path.real_paths() else {
            return Ok(vec![]);
        };

        for path in paths {
            let logfile = self.parse_single(path)?;
            logfiles.push(logfile);
        }

        logfiles.sort_by_key(|x| x.header.pid);
        Ok(logfiles)
    }
}

// TODO: CLEANUP
// impl LogfileSummary {
//     fn raw_into_tool_run(self) -> ToolRunSummary {
//         ToolRunSummary {
//             command: self.command.to_string_lossy().to_string(),
//             old_pid: None,
//             old_parent_pid: None,
//             pid: None,
//             parent_pid: None,
//             details: (!self.details.is_empty()).then(|| self.details.join("\n")),
//             log_path: self.log_path,
//             costs_summary: CostsSummaryType::default(),
//         }
//     }
//
//     pub fn old_into_tool_run(self) -> ToolRunSummary {
//         let costs_summary = match self.costs {
//             CostsKind::None => CostsSummaryType::None,
//             CostsKind::DhatCosts(ref costs) => {
//
// CostsSummaryType::DhatSummary(CostsSummary::new(EitherOrBoth::Right(costs.clone())))
// }             CostsKind::ErrorCosts(ref costs) =>
// CostsSummaryType::ErrorSummary(CostsSummary::new(
// EitherOrBoth::Right(costs.clone()),             )),
//         };
//         let old_pid = Some(self.pid);
//         let old_parent_pid = self.parent_pid;
//         ToolRunSummary {
//             old_pid,
//             old_parent_pid,
//             costs_summary,
//             ..self.raw_into_tool_run()
//         }
//     }
//
//     pub fn new_into_tool_run(self) -> ToolRunSummary {
//         let costs_summary = match self.costs {
//             CostsKind::DhatCosts(ref costs) => {
//
// CostsSummaryType::DhatSummary(CostsSummary::new(EitherOrBoth::Left(costs.clone())))             }
//             CostsKind::ErrorCosts(ref costs) => {
//
// CostsSummaryType::ErrorSummary(CostsSummary::new(EitherOrBoth::Left(costs.clone())))
// }             CostsKind::None => CostsSummaryType::None,
//         };
//         let pid = Some(self.pid);
//         let parent_pid = self.parent_pid;
//         ToolRunSummary {
//             pid,
//             parent_pid,
//             costs_summary,
//             ..self.raw_into_tool_run()
//         }
//     }
//
//     pub fn merge(self, old: &LogfileSummary) -> ToolRunSummary {
//         assert_eq!(self.command, old.command);
//         let costs_summary = match (&self.costs, &old.costs) {
//             (CostsKind::None, CostsKind::None) => CostsSummaryType::None,
//             (CostsKind::DhatCosts(new), CostsKind::DhatCosts(old)) => {
//                 CostsSummaryType::DhatSummary(CostsSummary::new(EitherOrBoth::Both((
//                     new.clone(),
//                     old.clone(),
//                 ))))
//             }
//             (CostsKind::ErrorCosts(new), CostsKind::ErrorCosts(old)) => {
//                 CostsSummaryType::ErrorSummary(CostsSummary::new(EitherOrBoth::Both((
//                     new.clone(),
//                     old.clone(),
//                 ))))
//             }
//             (CostsKind::None, CostsKind::DhatCosts(old)) => {
//
// CostsSummaryType::DhatSummary(CostsSummary::new(EitherOrBoth::Right(old.clone())))             }
//             (CostsKind::None, CostsKind::ErrorCosts(old)) => {
//
// CostsSummaryType::ErrorSummary(CostsSummary::new(EitherOrBoth::Right(old.clone())))             }
//             (CostsKind::DhatCosts(new), CostsKind::None) => {
//                 CostsSummaryType::DhatSummary(CostsSummary::new(EitherOrBoth::Left(new.clone())))
//             }
//             (CostsKind::ErrorCosts(new), CostsKind::None) => {
//
// CostsSummaryType::ErrorSummary(CostsSummary::new(EitherOrBoth::Left(new.clone())))             }
//             _ => panic!("The logfile summary types of new and old costs have to match"),
//         };
//
//         let old_pid = Some(old.pid);
//         let old_parent_pid = old.parent_pid;
//         let pid = Some(self.pid);
//         let parent_pid = self.parent_pid;
//         ToolRunSummary {
//             old_pid,
//             old_parent_pid,
//             pid,
//             parent_pid,
//             costs_summary,
//             ..self.raw_into_tool_run()
//         }
//     }
// }

pub fn extract_pid(line: &str) -> i32 {
    // TODO: Return error instead of unwraps
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
    // TODO: RENAME TO PARSER FACTORY and don't put this into ValgrindTool
    pub fn to_parser(self, root_dir: PathBuf) -> Box<dyn LogfileParser> {
        match self {
            ValgrindTool::DHAT => Box::new(DhatLogfileParser { root_dir }),
            ValgrindTool::Memcheck | ValgrindTool::DRD | ValgrindTool::Helgrind => {
                Box::new(ErrorMetricLogfileParser { root_dir })
            }
            _ => Box::new(GenericLogfileParser { root_dir }),
        }
    }
}

// TODO: CLEANUP
// impl LogfileSummary {
//     pub fn has_errors(&self) -> bool {
//         match &self.costs {
//             CostsKind::None | CostsKind::DhatCosts(_) => false,
//             CostsKind::ErrorCosts(costs) => costs
//                 .cost_by_kind(&ErrorMetricKind::Errors)
//                 .map_or(false, |e| e > 0),
//         }
//     }
// }
