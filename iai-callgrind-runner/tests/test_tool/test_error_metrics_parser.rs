use std::path::PathBuf;

use iai_callgrind_runner::api::ErrorMetricKind;
use iai_callgrind_runner::runner::metrics::Metrics;
use iai_callgrind_runner::runner::summary::ToolMetrics;
use iai_callgrind_runner::runner::tool::error_metric_parser::ErrorMetricLogfileParser;
use iai_callgrind_runner::runner::tool::logfile_parser::Parser;
use iai_callgrind_runner::runner::tool::{ToolOutputPathKind, ValgrindTool};
use pretty_assertions::assert_eq;
use rstest::rstest;

use crate::common::Fixtures;

/// The tests for the drd error metrics parser can be seen as exemplary for error metrics of other
/// tools like helgrind. Memcheck is tested separately.
#[rstest]
#[case::zero_errors("errors_all_zero", [0, 0, 0, 0])]
#[case::with_errors("with_errors", [12, 34, 56, 78])]
#[case::with_multiple_error_lines("with_two_error_lines", [12, 34, 56, 78])]
fn test_drd_error_metric_parser(#[case] fixture: &str, #[case] expected: [u64; 4]) {
    let metrics = Metrics::with_metric_kinds([
        (ErrorMetricKind::Errors, expected[0]),
        (ErrorMetricKind::Contexts, expected[1]),
        (ErrorMetricKind::SuppressedErrors, expected[2]),
        (ErrorMetricKind::SuppressedContexts, expected[3]),
    ]);
    let expected_metrics = ToolMetrics::ErrorMetrics(metrics);

    let drd_output_path =
        Fixtures::get_tool_output_path("drd", ValgrindTool::DRD, ToolOutputPathKind::Log, fixture);

    let parser = ErrorMetricLogfileParser {
        output_path: drd_output_path.clone(),
        root_dir: PathBuf::from("/does/not/matter"),
    };

    let logfiles = parser.parse().unwrap();
    assert_eq!(logfiles.len(), 1);
    assert_eq!(logfiles[0].metrics, expected_metrics);
}

#[test]
fn test_drd_error_metric_parser_when_multiple_pids() {
    let first_metrics = Metrics::with_metric_kinds([
        (ErrorMetricKind::Errors, 0),
        (ErrorMetricKind::Contexts, 0),
        (ErrorMetricKind::SuppressedErrors, 0),
        (ErrorMetricKind::SuppressedContexts, 0),
    ]);
    let expected_first_metrics = ToolMetrics::ErrorMetrics(first_metrics);
    let second_metrics = Metrics::with_metric_kinds([
        (ErrorMetricKind::Errors, 1),
        (ErrorMetricKind::Contexts, 23),
        (ErrorMetricKind::SuppressedErrors, 345),
        (ErrorMetricKind::SuppressedContexts, 4567),
    ]);
    let expected_second_metrics = ToolMetrics::ErrorMetrics(second_metrics);

    let drd_output_path = Fixtures::get_tool_output_path(
        "drd",
        ValgrindTool::DRD,
        ToolOutputPathKind::Log,
        "multiple_pids",
    );

    let parser = ErrorMetricLogfileParser {
        output_path: drd_output_path.to_log_output(),
        root_dir: PathBuf::from("/does/not/matter"),
    };

    let logfiles = parser.parse().unwrap();
    assert_eq!(logfiles.len(), 2);
    assert_eq!(logfiles[0].metrics, expected_first_metrics);
    assert_eq!(logfiles[1].metrics, expected_second_metrics);
}

/// Memcheck is tested separately because the content of the log files can differ greatly from drd
/// log files, although the `ERROR SUMMARY` line is the same.
#[rstest]
#[case::zero_errors("without_errors", [0, 0, 0, 0])]
#[case::bad_memory("bad_memory", [2, 2, 0, 0])]
#[case::with_errors("with_many_errors", [12, 34, 56, 78])]
#[case::with_multiple_error_lines("with_multiple_error_lines", [44, 555, 6666, 77777])]
fn test_memcheck_error_metric_parser(#[case] fixture: &str, #[case] expected: [u64; 4]) {
    let metrics = Metrics::with_metric_kinds([
        (ErrorMetricKind::Errors, expected[0]),
        (ErrorMetricKind::Contexts, expected[1]),
        (ErrorMetricKind::SuppressedErrors, expected[2]),
        (ErrorMetricKind::SuppressedContexts, expected[3]),
    ]);
    let expected_metrics = ToolMetrics::ErrorMetrics(metrics);

    let memcheck_output_path = Fixtures::get_tool_output_path(
        "memcheck",
        ValgrindTool::Memcheck,
        ToolOutputPathKind::Log,
        fixture,
    );

    let parser = ErrorMetricLogfileParser {
        output_path: memcheck_output_path.clone(),
        root_dir: PathBuf::from("/does/not/matter"),
    };

    let logfiles = parser.parse().unwrap();
    assert_eq!(logfiles.len(), 1);
    assert_eq!(logfiles[0].metrics, expected_metrics);
}

#[test]
fn test_memcheck_error_metric_parser_when_multiple_pids() {
    let first_metrics = Metrics::with_metric_kinds([
        (ErrorMetricKind::Errors, 0),
        (ErrorMetricKind::Contexts, 0),
        (ErrorMetricKind::SuppressedErrors, 0),
        (ErrorMetricKind::SuppressedContexts, 0),
    ]);
    let expected_first_metrics = ToolMetrics::ErrorMetrics(first_metrics);
    let second_metrics = Metrics::with_metric_kinds([
        (ErrorMetricKind::Errors, 11),
        (ErrorMetricKind::Contexts, 222),
        (ErrorMetricKind::SuppressedErrors, 3333),
        (ErrorMetricKind::SuppressedContexts, 44444),
    ]);
    let expected_second_metrics = ToolMetrics::ErrorMetrics(second_metrics);

    let memcheck_output_path = Fixtures::get_tool_output_path(
        "memcheck",
        ValgrindTool::Memcheck,
        ToolOutputPathKind::Log,
        "multiple_pids",
    );

    let parser = ErrorMetricLogfileParser {
        output_path: memcheck_output_path.clone(),
        root_dir: PathBuf::from("/does/not/matter"),
    };

    let logfiles = parser.parse().unwrap();
    assert_eq!(logfiles.len(), 2);
    assert_eq!(logfiles[0].metrics, expected_first_metrics);
    assert_eq!(logfiles[1].metrics, expected_second_metrics);
}
