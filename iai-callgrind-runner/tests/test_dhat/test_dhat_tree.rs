use iai_callgrind_runner::api::EntryPoint;
use iai_callgrind_runner::runner::dhat::json_parser::parse;
use iai_callgrind_runner::runner::dhat::model::DhatData;
use iai_callgrind_runner::runner::dhat::tree::{DhatTree, Tree};

use crate::common::Fixtures;

#[test]
fn test_dhat_tree() {
    let path = Fixtures::get_path_of("dhat/dhat.minimal.out");
    let data: DhatData = parse(&path).unwrap();
    // TODO: CONTINUE
    let _tree = DhatTree::from_json(data, &EntryPoint::None, &[]);
}
