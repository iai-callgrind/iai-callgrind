use iai_callgrind_runner::api::{DhatMetric, EntryPoint};
use iai_callgrind_runner::runner::dhat::json_parser::parse;
use iai_callgrind_runner::runner::dhat::model::{DhatData, Mode};
use iai_callgrind_runner::runner::dhat::tree::{Data, DhatTree, Tree};
use iai_callgrind_runner::runner::metrics::Metrics;
use iai_callgrind_runner::runner::summary::ToolMetrics;
use iai_callgrind_runner::util::Glob;
use pretty_assertions::assert_eq;

use crate::common::Fixtures;

fn data_fixture_pps_bench_func() -> Data {
    Data {
        total_bytes: 472,
        total_blocks: 1,
        total_lifetimes: Some(157_144),
        maximum_bytes: Some(472),
        maximum_blocks: Some(1),
        bytes_at_max: Some(472),
        blocks_at_max: Some(1),
        bytes_at_end: Some(0),
        blocks_at_end: Some(0),
        blocks_read: Some(2858),
        blocks_write: Some(1347),
    }
}

fn data_fixture_pps_calloc() -> Data {
    Data {
        total_bytes: 152,
        total_blocks: 1,
        total_lifetimes: Some(281),
        maximum_bytes: Some(0),
        maximum_blocks: Some(0),
        bytes_at_max: Some(0),
        blocks_at_max: Some(0),
        bytes_at_end: Some(0),
        blocks_at_end: Some(0),
        blocks_read: Some(24),
        blocks_write: Some(16),
    }
}

fn data_fixture_pps_malloc() -> Data {
    Data {
        total_bytes: 360,
        total_blocks: 2,
        total_lifetimes: Some(156_245),
        maximum_bytes: Some(240),
        maximum_blocks: Some(1),
        bytes_at_max: Some(240),
        blocks_at_max: Some(1),
        bytes_at_end: Some(0),
        blocks_at_end: Some(0),
        blocks_read: Some(6188),
        blocks_write: Some(4927),
    }
}

#[test]
fn test_dhat_tree_when_ad_hoc_mode() {
    let data = Data {
        total_bytes: 15,
        total_blocks: 1,
        total_lifetimes: None,
        maximum_bytes: None,
        maximum_blocks: None,
        bytes_at_max: None,
        blocks_at_max: None,
        bytes_at_end: None,
        blocks_at_end: None,
        blocks_read: None,
        blocks_write: None,
    };
    let mut expected_tree = DhatTree::default();
    expected_tree.set_mode(Mode::AdHoc);
    expected_tree.insert(&[1, 2, 3, 4], &data);

    let mut metrics = Metrics::empty();
    metrics.insert_all(&[
        (DhatMetric::TotalUnits, 15.into()),
        (DhatMetric::TotalEvents, 1.into()),
    ]);
    let expected_metrics = ToolMetrics::Dhat(metrics);

    let path = Fixtures::get_path_of("dhat/dhat.ad_hoc_mode.out");
    let data: DhatData = parse(&path).unwrap();
    let actual = DhatTree::from_json(data, &EntryPoint::None, &[]);

    assert_eq!(actual, expected_tree);
    assert_eq!(actual.metrics(), expected_metrics);
}

#[test]
fn test_dhat_tree_when_copy_mode() {
    let data = Data {
        total_bytes: 20,
        total_blocks: 1,
        total_lifetimes: None,
        maximum_bytes: None,
        maximum_blocks: None,
        bytes_at_max: None,
        blocks_at_max: None,
        bytes_at_end: None,
        blocks_at_end: None,
        blocks_read: None,
        blocks_write: None,
    };
    let mut expected_tree = DhatTree::default();
    expected_tree.set_mode(Mode::Copy);
    expected_tree.insert(&[1, 2, 3, 4], &data);

    let mut metrics = Metrics::empty();
    metrics.insert_all(&[
        (DhatMetric::TotalBytes, 20.into()),
        (DhatMetric::TotalBlocks, 1.into()),
    ]);
    let expected_metrics = ToolMetrics::Dhat(metrics);

    let path = Fixtures::get_path_of("dhat/dhat.copy_mode.out");
    let data: DhatData = parse(&path).unwrap();
    let actual = DhatTree::from_json(data, &EntryPoint::Default, &[]);

    assert_eq!(actual, expected_tree);
    assert_eq!(actual.metrics(), expected_metrics);
}

#[test]
fn test_dhat_tree_when_entry_point_and_frames() {
    let mut expected = DhatTree::default();
    expected.insert(&[1, 2, 3, 4], &data_fixture_pps_bench_func());
    expected.insert(&[1], &data_fixture_pps_malloc());

    let path = Fixtures::get_path_of("dhat/dhat.with_entry_point.out");
    let data: DhatData = parse(&path).unwrap();
    let actual = DhatTree::from_json(data, &EntryPoint::Default, &[Glob::new("malloc")]);

    assert_eq!(actual, expected);
}

#[test]
fn test_dhat_tree_when_entry_point_and_no_frames() {
    let mut expected = DhatTree::default();
    expected.insert(&[1, 2, 3, 4], &data_fixture_pps_bench_func());

    let path = Fixtures::get_path_of("dhat/dhat.with_entry_point.out");
    let data: DhatData = parse(&path).unwrap();
    let actual = DhatTree::from_json(data, &EntryPoint::Default, &[]);

    assert_eq!(actual, expected);
}

#[test]
fn test_dhat_tree_when_entry_point_custom_and_frames() {
    let mut expected = DhatTree::default();
    expected.insert(&[1, 2, 3, 4], &data_fixture_pps_bench_func());
    expected.insert(&[5], &data_fixture_pps_calloc());

    let path = Fixtures::get_path_of("dhat/dhat.with_entry_point.out");
    let data: DhatData = parse(&path).unwrap();
    let actual = DhatTree::from_json(
        data,
        &EntryPoint::Custom("test_dhat::*".to_owned()),
        &[Glob::new("calloc")],
    );

    assert_eq!(actual, expected);
}

#[test]
fn test_dhat_tree_when_entry_point_custom_no_frames() {
    let mut expected = DhatTree::default();
    expected.insert(&[1, 2, 3, 4], &data_fixture_pps_bench_func());

    let path = Fixtures::get_path_of("dhat/dhat.with_entry_point.out");
    let data: DhatData = parse(&path).unwrap();
    let actual = DhatTree::from_json(data, &EntryPoint::Custom("test_dhat::*".to_owned()), &[]);

    assert_eq!(actual, expected);
}

#[test]
fn test_dhat_tree_when_no_entry_point_but_frames() {
    let mut expected = DhatTree::default();
    expected.insert(&[1, 2, 3, 4], &data_fixture_pps_bench_func());

    let path = Fixtures::get_path_of("dhat/dhat.with_entry_point.out");
    let data: DhatData = parse(&path).unwrap();
    let actual = DhatTree::from_json(data, &EntryPoint::None, &[Glob::new("test_dhat::tool::*")]);

    assert_eq!(actual, expected);
}

#[test]
fn test_dhat_tree_when_no_entry_point_no_frames() {
    let mut expected = DhatTree::default();
    expected.insert(&[1, 2, 3, 4], &data_fixture_pps_bench_func());
    expected.insert(&[1], &data_fixture_pps_malloc());
    expected.insert(&[5], &data_fixture_pps_calloc());

    let path = Fixtures::get_path_of("dhat/dhat.with_entry_point.out");
    let data: DhatData = parse(&path).unwrap();
    let actual = DhatTree::from_json(data, &EntryPoint::None, &[]);

    assert_eq!(actual, expected);
}
