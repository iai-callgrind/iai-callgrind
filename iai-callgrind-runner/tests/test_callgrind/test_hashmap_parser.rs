use std::path::Path;

use iai_callgrind_runner::runner::callgrind::hashmap_parser::HashMapParser;
use iai_callgrind_runner::runner::callgrind::CallgrindParser;
use iai_callgrind_runner::runner::IaiCallgrindError;

use crate::common::get_callgrind_output;

fn assert_parse_error<T>(file: &Path, result: Result<T, IaiCallgrindError>, message: &str)
where
    T: std::cmp::PartialEq + std::fmt::Debug,
{
    assert_eq!(
        result,
        Err(IaiCallgrindError::ParseError((
            file.to_owned(),
            message.to_owned()
        )))
    );
}

#[test]
fn test_when_version_mismatch_then_should_return_error() {
    let mut parser = HashMapParser::default();
    let output = get_callgrind_output("callgrind.out/invalid.version_too_high.out");
    assert_parse_error(
        &output.file,
        parser.parse(&output),
        "Version mismatch: Requires version '1' but was '2'",
    );
}

#[test]
fn test_when_empty_file_then_should_return_error() {
    let mut parser = HashMapParser::default();
    let output = get_callgrind_output("callgrind.out/empty.out");
    assert_parse_error(&output.file, parser.parse(&output), "Empty file");
}

#[test]
fn test_when_no_records_and_no_summary_and_totals() {
    let mut parser = HashMapParser::default();
    let output = get_callgrind_output("callgrind.out/no_records.no_summary_and_totals.out");
    let expected_parser = HashMapParser::default();

    parser.parse(&output).unwrap();

    assert_eq!(parser, expected_parser);
}

#[test]
fn test_when_no_records_but_summary_and_totals() {
    let mut parser = HashMapParser::default();
    let output = get_callgrind_output("callgrind.out/no_records.with_summary_and_totals.out");
    let expected_parser = HashMapParser::default();

    parser.parse(&output).unwrap();
    assert_eq!(parser, expected_parser);
}
