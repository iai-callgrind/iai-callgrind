use iai_callgrind_runner::runner::callgrind::hashmap_parser::{CallgrindMap, HashMapParser};
use iai_callgrind_runner::runner::callgrind::parser::Parser;
use pretty_assertions::assert_eq;
use rstest::rstest;

use crate::common::{assert_parse_error, Fixtures};

#[test]
fn test_when_version_mismatch_then_should_return_error() {
    let parser = HashMapParser::default();
    let output = Fixtures::get_callgrind_output_path("callgrind.out/invalid.version_too_high.out");
    assert_parse_error(
        output.as_path(),
        parser.parse(&output),
        "Version mismatch: Requires callgrind format version '1' but was '2'",
    );
}

#[test]
fn test_when_empty_file_then_should_return_error() {
    let parser = HashMapParser::default();
    let output = Fixtures::get_callgrind_output_path("callgrind.out/empty.out");
    assert_parse_error(output.as_path(), parser.parse(&output), "Empty file");
}

#[test]
fn test_valid_just_main() {
    let parser = HashMapParser::default();
    let output = Fixtures::get_callgrind_output_path("callgrind.out/valid.minimal_main.out");
    let expected_map =
        Fixtures::load_serialized("callgrind.out/valid.minimal_main.exp_map").unwrap();

    let actual_map = parser.parse(&output).unwrap();

    assert_eq!(actual_map, expected_map);
}

#[rstest]
#[case::no_summary_and_totals("callgrind.out/no_records.no_summary_and_totals.out")]
#[case::summary_and_totals("callgrind.out/no_records.with_summary_and_totals.out")]
fn test_when_no_records(#[case] fixture: &str) {
    let parser = HashMapParser::default();
    let output = Fixtures::get_callgrind_output_path(fixture);
    let expected_map = CallgrindMap::default();

    let actual_map = parser.parse(&output).unwrap();

    assert_eq!(actual_map, expected_map);
}
