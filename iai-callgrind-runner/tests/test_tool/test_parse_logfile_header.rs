use std::io::{BufRead, BufReader};

use iai_callgrind_runner::runner::tool::logfile_parser::{parse_header, Header};
use iai_callgrind_runner::runner::tool::ValgrindTool;
use rstest::rstest;

use crate::common::Fixtures;

fn expected_header(command: &str, pid: i32, parent_pid: Option<i32>) -> Header {
    Header {
        command: command.to_owned(),
        pid,
        parent_pid,
    }
}

/// The basic structure of the logfile header is the same for all tools, so only drd is tested
/// exemplary here
#[rstest]
#[case::when_no_errors(
    "errors_all_zero",
    expected_header(
        "/home/some/workspace/target/release/deps/test_lib_bench_some-4c5214398e2f5bd1",
        1915454,
        Some(1915177)
    )
)]
// What comes after the header and if there are errors or not should not influence the resulting
// header
#[case::when_errors(
    "with_errors",
    expected_header(
        "/home/some/workspace/target/release/deps/test_lib_bench_some-4c5214398e2f5bd1",
        1915455,
        Some(1915178)
    )
)]
fn test_parse_logfile_header(#[case] name: &str, #[case] expected_header: Header) {
    use std::fs::File;

    use iai_callgrind_runner::runner::tool::ToolOutputPathKind;

    let tool_output_path =
        Fixtures::get_tool_output_path("drd", ValgrindTool::DRD, ToolOutputPathKind::Log, name);
    let mut logfile_headers = vec![];
    for path in tool_output_path.real_paths().unwrap() {
        let file = File::open(&path).unwrap();
        let reader = BufReader::new(file);

        let header = parse_header(&path, &mut reader.lines().map(Result::unwrap)).unwrap();
        logfile_headers.push(header);
    }

    assert_eq!(logfile_headers.len(), 1);
    assert_eq!(logfile_headers[0], expected_header);
}
