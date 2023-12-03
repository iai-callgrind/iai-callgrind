use iai_callgrind_runner::api::EventKind;
use iai_callgrind_runner::runner::callgrind::model::Costs;
use iai_callgrind_runner::runner::callgrind::parser::Parser;
use iai_callgrind_runner::runner::callgrind::summary_parser::SummaryParser;
use iai_callgrind_runner::runner::tool::{ToolOutputPathKind, ValgrindTool};
use rstest::rstest;

use crate::common::{assert_parse_error, Fixtures};

// Ir Dr Dw I1mr D1mr D1mw ILmr DLmr DLmw
#[rstest]
#[case::no_records("no_records.with_summary_and_totals", [0, 0, 0, 0, 0, 0, 0, 0, 0])]
#[case::with_records("no_entry_point", [325261, 78145, 35789, 1595, 2119, 850, 1558, 1485, 799])]
fn test_sentinel_parser(#[case] fixture: &str, #[case] costs: [u64; 9]) {
    let expected_costs = Costs::with_event_kinds([
        (EventKind::Ir, costs[0]),
        (EventKind::Dr, costs[1]),
        (EventKind::Dw, costs[2]),
        (EventKind::I1mr, costs[3]),
        (EventKind::D1mr, costs[4]),
        (EventKind::D1mw, costs[5]),
        (EventKind::ILmr, costs[6]),
        (EventKind::DLmr, costs[7]),
        (EventKind::DLmw, costs[8]),
    ]);

    let callgrind_output = Fixtures::get_tool_output_path(
        "callgrind.out",
        ValgrindTool::Callgrind,
        ToolOutputPathKind::Out,
        fixture,
    );

    let parser = SummaryParser;
    let actual_costs = parser.parse(&callgrind_output).unwrap();

    assert_eq!(actual_costs, expected_costs);
}

#[test]
fn test_summary_parser_when_not_found_then_error() {
    let callgrind_output = Fixtures::get_tool_output_path(
        "callgrind.out",
        ValgrindTool::Callgrind,
        ToolOutputPathKind::Out,
        "no_records.no_summary_and_totals",
    );

    let result = SummaryParser.parse(&callgrind_output);
    assert_parse_error(
        &callgrind_output.to_path(),
        result,
        "No summary or totals line found",
    )
}
