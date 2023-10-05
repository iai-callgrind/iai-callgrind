use std::path::Path;

use anyhow::Result;
use iai_callgrind_runner::runner::callgrind::hashmap_parser::{CallgrindMap, HashMapParser};
use iai_callgrind_runner::runner::callgrind::parser::Parser;
use pretty_assertions::assert_eq;
use rstest::rstest;

use crate::common::{get_callgrind_output, load_serialized};

fn assert_parse_error<T>(file: &Path, result: Result<T>, message: &str)
where
    T: std::cmp::PartialEq + std::fmt::Debug,
{
    assert_eq!(
        result.unwrap_err().to_string(),
        format!("Error parsing file '{}': {message}", file.display())
    );
}

#[test]
fn test_when_version_mismatch_then_should_return_error() {
    let parser = HashMapParser::default();
    let output = get_callgrind_output("callgrind.out/invalid.version_too_high.out");
    assert_parse_error(
        &output.path,
        parser.parse(&output),
        "Version mismatch: Requires callgrind format version '1' but was '2'",
    );
}

#[test]
fn test_when_empty_file_then_should_return_error() {
    let parser = HashMapParser::default();
    let output = get_callgrind_output("callgrind.out/empty.out");
    assert_parse_error(&output.path, parser.parse(&output), "Empty file");
}

#[test]
fn test_valid_just_main() {
    let parser = HashMapParser::default();
    let output = get_callgrind_output("callgrind.out/valid.minimal_main.out");
    let expected_map = load_serialized("callgrind.out/valid.minimal_main.expected_map").unwrap();

    let actual_map = parser.parse(&output).unwrap();

    assert_eq!(actual_map, expected_map);
}

#[rstest]
#[case::no_summary_and_totals("callgrind.out/no_records.no_summary_and_totals.out")]
#[case::summary_and_totals("callgrind.out/no_records.with_summary_and_totals.out")]
fn test_when_no_records(#[case] fixture: &str) {
    let parser = HashMapParser::default();
    let output = get_callgrind_output(fixture);
    let expected_map = CallgrindMap::default();

    let actual_map = parser.parse(&output).unwrap();

    assert_eq!(actual_map, expected_map);
}
