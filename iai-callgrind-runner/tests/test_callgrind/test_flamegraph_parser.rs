use iai_callgrind_runner::runner::callgrind::flamegraph_parser::FlamegraphParser;
use iai_callgrind_runner::runner::callgrind::model::EventType;
use iai_callgrind_runner::runner::callgrind::parser::{Parser, Sentinel};
use pretty_assertions::assert_eq;

use crate::common::{get_callgrind_output, get_project_root, load_stacks};

#[test]
fn test_flamegraph_parser() {
    let output = get_callgrind_output("callgrind.out/callgrind.with_entry_point.out");
    let expected_stacks = load_stacks("callgrind.out/callgrind.with_entry_point.exp_stacks");
    let parser = FlamegraphParser::new(Some(&Sentinel::new("main")), get_project_root());

    let result = parser.parse(&output).unwrap();
    let stacks = result.to_stack_format(&EventType::Ir).unwrap();

    assert_eq!(stacks.len(), expected_stacks.len());
    for (index, (stack, expected_stack)) in stacks.iter().zip(expected_stacks.iter()).enumerate() {
        assert_eq!(stack, expected_stack, "Assertion failed at index '{index}'");
    }
}
