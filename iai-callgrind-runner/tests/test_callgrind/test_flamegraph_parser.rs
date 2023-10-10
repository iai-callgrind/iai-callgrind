use iai_callgrind_runner::runner::callgrind::flamegraph_parser::FlamegraphParser;
use iai_callgrind_runner::runner::callgrind::model::EventType;
use iai_callgrind_runner::runner::callgrind::parser::{Parser, Sentinel};
use pretty_assertions::assert_eq;
use rstest::rstest;

use crate::common::{get_callgrind_output, get_project_root, load_stacks};

#[rstest]
#[case::when_entry_point("when_entry_point", Some(Sentinel::new("benchmark_tests_exit::main")))]
#[case::no_entry_point("no_entry_point", None)]
fn test_flamegraph_parser_when_no_entry_point(
    #[case] fixture: &str,
    #[case] sentinel: Option<Sentinel>,
) {
    let output = get_callgrind_output(format!("callgrind.out/{fixture}.out"));
    let expected_stacks = load_stacks(format!("callgrind.out/{fixture}.exp_stacks"));
    let parser = FlamegraphParser::new(sentinel.as_ref(), get_project_root());

    let result = parser.parse(&output).unwrap();
    let stacks = result.to_stack_format(&EventType::Ir).unwrap();

    assert_eq!(stacks.len(), expected_stacks.len());
    // Assert line by line or else the output on error is unreadable
    for (index, (stack, expected_stack)) in stacks.iter().zip(expected_stacks.iter()).enumerate() {
        assert_eq!(
            stack, expected_stack,
            "Assertion failed at line index '{index}'"
        );
    }
}
