use iai_callgrind_runner::runner::dhat::json_parser::parse;
use iai_callgrind_runner::runner::dhat::model::{DhatData, Frame, Mode, ProgramPoint};
use pretty_assertions::assert_eq;

use crate::common::Fixtures;

fn dhat_data_fixture() -> DhatData {
    DhatData {
        dhat_file_version: 2,
        mode: Mode::Heap,
        verb: "Allocated".to_owned(),
        has_block_lifetimes: true,
        has_block_accesses: true,
        byte_unit: None,
        bytes_unit: None,
        block_unit: None,
        time_unit: "instrs".to_owned(),
        time_unit_m: "Minstr".to_owned(),
        time_threshold: Some(500),
        command: "/some/path/bench-bb025b17fd65eb7d --iai-run my_group 0 2 file::group::function"
            .to_owned(),
        pid: 2,
        time_end: 500_000,
        time_global_max: Some(160_000),
        program_points: vec![],
        frame_table: vec![Frame::Root],
    }
}

fn program_point_fixture() -> ProgramPoint {
    ProgramPoint {
        total_bytes: 1024,
        total_blocks: 1,
        total_lifetimes: Some(160_000),
        maximum_bytes: Some(1024),
        maximum_blocks: Some(1),
        bytes_at_max: Some(1024),
        blocks_at_max: Some(1),
        bytes_at_end: Some(0),
        blocks_at_end: Some(0),
        blocks_read: Some(9456),
        blocks_write: Some(5093),
        accesses: Some(vec![20, -751, 15, -245, 12, -27, 12]),
        frames: vec![1, 2],
    }
}

#[test]
fn test_serde() {
    let path = Fixtures::get_path_of("dhat/dhat.minimal.out");

    let mut expected = dhat_data_fixture();
    expected.program_points.push(program_point_fixture());
    expected.frame_table.push(
        "0x48C67A8: malloc (in /usr/lib/valgrind/vgpreload_dhat-amd64-linux.so)"
            .parse()
            .unwrap(),
    );

    let actual: DhatData = parse(&path).unwrap();

    assert_eq!(actual, expected);
}
