use std::path::PathBuf;

use iai_callgrind_runner::api::DhatMetricKind;
use iai_callgrind_runner::runner::costs::Costs;
use iai_callgrind_runner::runner::dhat::logfile_parser::DhatLogfileParser;
use iai_callgrind_runner::runner::summary::{
    CostsKind, CostsSummary, CostsSummaryType, ToolRunSummary,
};
use iai_callgrind_runner::runner::tool::logfile_parser::{LogfileParser, LogfileSummary};
use iai_callgrind_runner::util::EitherOrBoth;

fn dummy_cost(cost: u64) -> Costs<DhatMetricKind> {
    Costs::with_event_kinds([(DhatMetricKind::TotalBytes, cost)])
}

fn dummy_summary(cmd: &str, pid: i32, cost: u64) -> LogfileSummary {
    LogfileSummary {
        command: cmd.parse().unwrap(),
        pid,
        parent_pid: Some(pid + 1),
        details: vec![],
        log_path: PathBuf::new(),
        costs: CostsKind::DhatCosts(dummy_cost(cost)),
    }
}

fn dummy_tool_run_summary(
    cmd: &str,
    pid: Option<i32>,
    old_pid: Option<i32>,
    cost: Option<u64>,
    old_cost: Option<u64>,
) -> ToolRunSummary {
    let costs_summary = match (cost, old_cost) {
        (None, None) => panic!("new or old cost must be present"),
        (Some(new_cost), None) => CostsSummary::new(EitherOrBoth::Left(dummy_cost(new_cost))),
        (None, Some(old_cost)) => CostsSummary::new(EitherOrBoth::Right(dummy_cost(old_cost))),
        (Some(new_cost), Some(old_cost)) => CostsSummary::new(EitherOrBoth::Both((
            dummy_cost(new_cost),
            dummy_cost(old_cost),
        ))),
    };

    ToolRunSummary {
        command: cmd.to_string(),
        old_pid,
        old_parent_pid: old_pid.map(|x| x + 1),
        pid,
        parent_pid: pid.map(|x| x + 1),
        details: None,
        log_path: Default::default(),
        costs_summary: CostsSummaryType::DhatSummary(costs_summary),
    }
}

#[test]
fn test_dhat_merge_logfile_summaries() {
    let s1 = || dummy_summary("cmd1", 1, 10);
    let s1b = || dummy_summary("cmd1", 2, 20);
    let s2 = || dummy_summary("cmd2", 3, 30);
    let s1s1b = || dummy_tool_run_summary("cmd1", Some(1), Some(2), Some(10), Some(20));
    let s2n = || dummy_tool_run_summary("cmd2", Some(3), None, Some(30), None);
    let s2o = || dummy_tool_run_summary("cmd2", None, Some(3), None, Some(30));
    let s1bo = || dummy_tool_run_summary("cmd1", None, Some(2), None, Some(20));

    let dhat = DhatLogfileParser {
        root_dir: PathBuf::new(),
    };
    assert_eq!(
        dhat.merge_logfile_summaries(vec![s1b()], vec![s1()]),
        vec![s1s1b()]
    );
    assert_eq!(
        dhat.merge_logfile_summaries(vec![s1b()], vec![s1(), s2()]),
        vec![s1s1b(), s2n()]
    );
    assert_eq!(
        dhat.merge_logfile_summaries(vec![s1b(), s2()], vec![s1()]),
        vec![s1s1b(), s2o()]
    );
    assert_eq!(
        dhat.merge_logfile_summaries(vec![s1b()], vec![s2()]),
        vec![s1bo(), s2n()]
    );
}
