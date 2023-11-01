use iai_callgrind_runner::api::EventKind;
use iai_callgrind_runner::runner::callgrind::model::Costs;
use iai_callgrind_runner::runner::callgrind::parser::{Parser, Sentinel};
use iai_callgrind_runner::runner::callgrind::sentinel_parser::SentinelParser;
use rstest::rstest;

use crate::common::{assert_parse_error, Fixtures};

// Ir Dr Dw I1mr D1mr D1mw ILmr DLmr DLmw
#[rstest]
#[case::main("main", [102969, 23011, 13344, 579, 115, 88, 552, 52, 68])]
#[case::rust_func_with_many_cost_lines("std::rt::lang_start_internal", [102961, 23010, 13341, 577, 114, 88, 550, 52, 68])]
#[case::address("0x0000000000009560", [90863, 26797, 8566, 45, 736, 7, 45, 413, 4])]
#[case::benchmark_tests_exit_main("benchmark_tests_exit::main", [3473, 889, 559, 143, 30, 7, 116, 4, 4])]
#[case::single_cost_line("strcpy", [11, 4, 0, 2, 0, 0, 2, 0, 0])]
#[case::multiple_files_single_fn("__cpu_indicator_init@GCC_4.8.0", [346, 17, 33, 33, 4, 0, 33, 2, 0])]
fn test_sentinel_parser(#[case] sentinel: &str, #[case] costs: [u64; 9]) {
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
    let callgrind_output = Fixtures::get_callgrind_output("callgrind.out/no_entry_point.out");

    let parser = SentinelParser::new(&Sentinel::new(sentinel));
    let actual_costs = parser.parse(&callgrind_output).unwrap();

    assert_eq!(actual_costs, expected_costs);
}

#[test]
fn test_sentinel_parser_when_not_found_then_error() {
    let callgrind_output = Fixtures::get_callgrind_output("callgrind.out/no_entry_point.out");
    let sentinel = Sentinel::new("doesnotexist");

    let result = SentinelParser::new(&sentinel).parse(&callgrind_output);

    assert_parse_error(
        callgrind_output.as_path(),
        result,
        "Sentinel 'doesnotexist' not found",
    )
}
