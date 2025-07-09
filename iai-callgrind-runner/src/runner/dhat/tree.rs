use std::cmp::Ordering;
use std::ops::Add;

use lazy_static::lazy_static;
use polonius_the_crab::{polonius, ForLt, PoloniusResult};
use regex::Regex;

use super::model::{DhatData, Frame, ProgramPoint};
use crate::api::{DhatMetric, EntryPoint};
use crate::runner::metrics::Metrics;
use crate::runner::summary::ToolMetrics;
use crate::runner::DEFAULT_TOGGLE_RE;
use crate::util::glob_to_regex;

lazy_static! {
    static ref GLOB_TO_REGEX_RE: Regex = regex::Regex::new(r"([*])").expect("Regex should compile");
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Data {
    total_bytes: u64,
    total_blocks: u64,
    total_lifetimes: Option<u128>,
    maximum_bytes: Option<u64>,
    maximum_blocks: Option<u64>,
    bytes_at_max: Option<u64>,
    blocks_at_max: Option<u64>,
    bytes_at_end: Option<u64>,
    blocks_at_end: Option<u64>,
    blocks_read: Option<u64>,
    blocks_write: Option<u64>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Node {
    prefix: Vec<usize>,
    children: Vec<Node>,
    data: Data,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Tree {
    root: Box<Node>,
}

impl Data {
    fn zero() -> Self {
        Self {
            total_bytes: 0,
            total_blocks: 0,
            total_lifetimes: Some(0),
            maximum_bytes: Some(0),
            maximum_blocks: Some(0),
            bytes_at_max: Some(0),
            blocks_at_max: Some(0),
            bytes_at_end: Some(0),
            blocks_at_end: Some(0),
            blocks_read: Some(0),
            blocks_write: Some(0),
        }
    }

    fn add(&mut self, other: &Self) {
        self.total_bytes += other.total_bytes;
        self.total_blocks += other.total_blocks;
        self.total_lifetimes = sum_options(self.total_lifetimes, other.total_lifetimes);
        self.maximum_bytes = sum_options(self.maximum_bytes, other.maximum_bytes);
        self.maximum_blocks = sum_options(self.maximum_blocks, other.maximum_blocks);
        self.bytes_at_max = sum_options(self.bytes_at_max, other.bytes_at_max);
        self.blocks_at_max = sum_options(self.blocks_at_max, other.blocks_at_max);
        self.bytes_at_end = sum_options(self.bytes_at_end, other.bytes_at_end);
        self.blocks_at_end = sum_options(self.blocks_at_end, other.blocks_at_end);
        self.blocks_read = sum_options(self.blocks_read, other.blocks_read);
        self.blocks_write = sum_options(self.blocks_write, other.blocks_write);
    }
}

impl From<&ProgramPoint> for Data {
    fn from(value: &ProgramPoint) -> Self {
        Self {
            total_bytes: value.total_bytes,
            total_blocks: value.total_blocks,
            total_lifetimes: value.total_lifetimes,
            maximum_bytes: value.maximum_bytes,
            maximum_blocks: value.maximum_blocks,
            bytes_at_max: value.bytes_at_max,
            blocks_at_max: value.blocks_at_max,
            bytes_at_end: value.bytes_at_end,
            blocks_at_end: value.blocks_at_end,
            blocks_read: value.blocks_read,
            blocks_write: value.blocks_write,
        }
    }
}

impl Node {
    pub fn new(prefix: Vec<usize>, children: Vec<Node>, data: Data) -> Self {
        Self {
            prefix,
            children,
            data,
        }
    }

    pub fn with_prefix(prefix: Vec<usize>) -> Self {
        Self {
            prefix,
            children: Vec::default(),
            data: Data::default(),
        }
    }

    fn add_child(&mut self, prefix: &[usize], data: &Data) {
        self.children
            .push(Node::new(prefix.to_vec(), vec![], data.clone()));
    }

    fn find_child(&mut self, num: usize) -> Option<&mut Self> {
        self.children
            .iter_mut()
            .find(|node| node.prefix.first().is_some_and(|a| *a == num))
    }

    fn split(&mut self, index: usize, data: &Data) {
        let node = Node::new(
            self.prefix.split_off(index),
            std::mem::take(&mut self.children),
            self.data.clone(),
        );
        self.add_data(data);

        self.children.push(node);
    }

    fn split_index(&self, other: &[usize]) -> Option<usize> {
        let length = self.prefix.len().min(other.len());
        (0..length).find(|&index| self.prefix[index] != other[index])
    }

    fn add_data(&mut self, data: &Data) {
        self.data.add(data);
    }
}

impl Tree {
    pub fn from_json(dhat_data: DhatData, entry_point: &EntryPoint, frames: &[Regex]) -> Self {
        let mut matchers = frames.iter().collect::<Vec<_>>();
        let regex = match entry_point {
            EntryPoint::None => None,
            EntryPoint::Default => Regex::new(DEFAULT_TOGGLE_RE).ok(),
            EntryPoint::Custom(custom) => glob_to_regex(custom).ok(),
        };

        if let Some(regex) = &regex {
            matchers.push(regex);
        }

        let mut indices = vec![];
        for (index, frame) in dhat_data.frame_table.iter().enumerate() {
            if let Frame::Leaf(_, func_name, _) = frame {
                for matcher in &matchers {
                    if matcher.is_match(func_name) {
                        indices.push(index);
                    }
                }
            }
        }

        // TODO: It is overkill to build a real tree just for the root data.
        let mut tree = Tree::default();
        // This is the default behaviour
        if *entry_point == EntryPoint::None && frames.is_empty() {
            for program_point in dhat_data.program_points {
                let data = Data::from(&program_point);
                tree.insert(&program_point.frames, &data);
            }
        // Indices can only be present if there is a match of the entry point or the frames
        } else if !indices.is_empty() {
            for program_point in dhat_data.program_points {
                if program_point.frames.iter().any(|f| indices.contains(f)) {
                    let data = Data::from(&program_point);
                    tree.insert(&program_point.frames, &data);
                }
            }
        } else {
            // If there was an entry point or frames configured but didn't match any indices, do
            // nothing
            tree.root.data = Data::zero();
        }

        tree
    }

    pub fn metrics(&self) -> ToolMetrics {
        // This is the same order as order of metrics in the log file output
        let metrics = [
            (DhatMetric::TotalBytes, Some(self.root.data.total_bytes)),
            (DhatMetric::TotalBlocks, Some(self.root.data.total_blocks)),
            (DhatMetric::AtTGmaxBytes, self.root.data.bytes_at_max),
            (DhatMetric::AtTGmaxBlocks, self.root.data.blocks_at_max),
            (DhatMetric::AtTEndBytes, self.root.data.bytes_at_end),
            (DhatMetric::AtTEndBlocks, self.root.data.blocks_at_end),
            (DhatMetric::ReadsBytes, self.root.data.blocks_read),
            (DhatMetric::WritesBytes, self.root.data.blocks_write),
            (
                DhatMetric::TotalLifetimes,
                #[allow(clippy::cast_possible_truncation)]
                self.root.data.total_lifetimes.map(|a| a as u64),
            ),
            (DhatMetric::MaximumBytes, self.root.data.maximum_bytes),
            (DhatMetric::MaximumBlocks, self.root.data.maximum_blocks),
        ];

        let mut tool_metrics = Metrics::empty();
        for (key, value) in metrics.iter().filter_map(|(a, b)| b.map(|b| (a, b))) {
            tool_metrics.insert(*key, value.into());
        }
        ToolMetrics::Dhat(tool_metrics)
    }

    pub fn insert(&mut self, prefix: &[usize], data: &Data) {
        let mut current = &mut *self.root;
        let mut index = 0;

        // root aggregates all data
        current.add_data(data);

        while index < prefix.len() {
            let key = prefix[index];
            let current_prefix = &prefix[index..];

            match polonius::<_, _, ForLt!(&mut Node)>(current, |current| {
                if let Some(child) = current.find_child(key) {
                    PoloniusResult::Borrowing(child)
                } else {
                    PoloniusResult::Owned(())
                }
            }) {
                PoloniusResult::Borrowing(child) => {
                    if let Some(split_index) = child.split_index(current_prefix) {
                        child.split(split_index, data);
                        index += split_index;
                    } else {
                        match current_prefix.len().cmp(&child.prefix.len()) {
                            Ordering::Less => {
                                child.split(current_prefix.len(), data);
                                return;
                            }
                            Ordering::Greater => {
                                child.add_data(data);
                                index += child.prefix.len();
                            }
                            Ordering::Equal => {
                                // TODO: Is this a real possibility with DHAT? Add or not?
                                child.add_data(data);
                                return;
                            }
                        }
                    }

                    current = child;
                }
                PoloniusResult::Owned {
                    input_borrow: current,
                    ..
                } => {
                    current.add_child(current_prefix, data);
                    return;
                }
            }
        }
    }
}

impl From<DhatData> for Tree {
    fn from(value: DhatData) -> Self {
        let mut tree = Tree::default();
        for pps in value.program_points {
            let data = Data::from(&pps);
            tree.insert(&pps.frames, &data);
        }

        tree
    }
}

fn sum_options<T: Add<Output = T>>(lhs: Option<T>, rhs: Option<T>) -> Option<T> {
    match (lhs, rhs) {
        (None, None) => None,
        (None, Some(b)) => Some(b),
        (Some(a), None) => Some(a),
        (Some(a), Some(b)) => Some(a + b),
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn tree_fixture() -> Tree {
        Tree {
            root: Box::new(Node::new(
                vec![],
                vec![Node::new(
                    vec![1, 2, 3],
                    vec![Node::new(vec![4, 5], vec![], data_fixture(1))],
                    data_fixture(2),
                )],
                data_fixture(2),
            )),
        }
    }

    fn data_fixture(num: u64) -> Data {
        Data {
            total_bytes: num,
            ..Default::default()
        }
    }

    #[test]
    fn test_insert_empty() {
        let mut expected = Tree::default();
        expected.root.data = data_fixture(1);

        let mut tree = Tree::default();
        tree.insert(&[], &data_fixture(1));

        assert_eq!(tree, expected);
    }

    #[test]
    fn test_insert_full_shorter() {
        let expected = Tree {
            root: Box::new(Node::new(
                vec![],
                vec![Node::new(
                    vec![1],
                    vec![Node::new(
                        vec![2, 3],
                        vec![Node::new(vec![4, 5], vec![], data_fixture(1))],
                        data_fixture(2),
                    )],
                    data_fixture(3),
                )],
                data_fixture(3),
            )),
        };

        let mut tree = tree_fixture();
        tree.insert(&[1], &data_fixture(1));

        assert_eq!(tree, expected);
    }

    #[test]
    fn test_insert_full_longer() {
        let expected = Tree {
            root: Box::new(Node::new(
                vec![],
                vec![Node::new(
                    vec![1, 2, 3],
                    vec![
                        Node::new(vec![4, 5], vec![], data_fixture(1)),
                        Node::new(vec![6], vec![], data_fixture(1)),
                    ],
                    data_fixture(3),
                )],
                data_fixture(3),
            )),
        };

        let mut tree = tree_fixture();
        tree.insert(&[1, 2, 3, 6], &data_fixture(1));

        assert_eq!(tree, expected);
    }

    #[test]
    fn test_insert_mixed() {
        let expected = Tree {
            root: Box::new(Node::new(
                vec![],
                vec![Node::new(
                    vec![1],
                    vec![
                        Node::new(
                            vec![2, 3],
                            vec![Node::new(vec![4, 5], vec![], data_fixture(1))],
                            data_fixture(2),
                        ),
                        Node::new(vec![6], vec![], data_fixture(1)),
                    ],
                    data_fixture(3),
                )],
                data_fixture(3),
            )),
        };

        let mut tree = tree_fixture();
        tree.insert(&[1, 6], &data_fixture(1));

        assert_eq!(tree, expected);
    }

    #[test]
    fn test_insert_no_match() {
        let expected = Tree {
            root: Box::new(Node::new(
                vec![],
                vec![
                    Node::new(
                        vec![1, 2, 3],
                        vec![Node::new(vec![4, 5], vec![], data_fixture(1))],
                        data_fixture(2),
                    ),
                    Node::new(vec![6], vec![], data_fixture(1)),
                ],
                data_fixture(3),
            )),
        };

        let mut tree = tree_fixture();
        tree.insert(&[6], &data_fixture(1));

        assert_eq!(tree, expected);
    }

    #[test]
    fn test_insert_equal() {
        let expected = Tree {
            root: Box::new(Node::new(
                vec![],
                vec![Node::new(
                    vec![1, 2, 3],
                    vec![Node::new(vec![4, 5], vec![], data_fixture(1))],
                    data_fixture(3),
                )],
                data_fixture(3),
            )),
        };

        let mut tree = tree_fixture();
        tree.insert(&[1, 2, 3], &data_fixture(1));

        assert_eq!(tree, expected);
    }
}
