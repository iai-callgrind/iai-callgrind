//! Module containing the dhat trees

use std::cmp::Ordering;
use std::ops::Add;

use polonius_the_crab::{polonius, ForLt, PoloniusResult};
use simplematch::DoWild;

use super::model::{DhatData, Frame, Mode, ProgramPoint};
use crate::api::{DhatMetric, EntryPoint};
use crate::runner::metrics::Metrics;
use crate::runner::summary::ToolMetrics;
use crate::runner::DEFAULT_TOGGLE;

/// The [`Data`] of each [`Node`]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Data {
    /// The blocks at t-end
    pub blocks_at_end: Option<u64>,
    /// The blocks at t-gmax
    pub blocks_at_max: Option<u64>,
    /// The reads of blocks
    pub blocks_read: Option<u64>,
    /// The writes of blocks
    pub blocks_write: Option<u64>,
    /// The bytes at t-end
    pub bytes_at_end: Option<u64>,
    /// The bytes at t-gmax
    pub bytes_at_max: Option<u64>,
    /// The maximum blocks
    pub maximum_blocks: Option<u64>,
    /// The maximum bytes
    pub maximum_bytes: Option<u64>,
    /// The total blocks
    pub total_blocks: u64,
    /// The total bytes
    pub total_bytes: u64,
    /// Total lifetimes of all blocks allocated
    pub total_lifetimes: Option<u128>,
}

/// A full-fledged dhat prefix tree
///
/// # Developers
///
/// This tree is currently not used but it is fully functional. However, only `insert` is
/// implemented to be able to build the tree but it may be needed to add methods like `remove`,
/// `lookup`, etc.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DhatTree {
    mode: Mode,
    root: Box<Node>,
}

/// The [`Node`] in a [`Tree`]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Node {
    children: Vec<Node>,
    data: Data,
    prefix: Vec<usize>,
}

/// A [`Tree`] without any leafs. Useful if only the root data and metrics are of interest.
///
/// If you're just interested in the data of the root then it is more performant to use this tree
/// instead of building a complete [`DhatTree`]. The dhat metrics of the root are the summarized
/// metrics of all its children, so all this [`Tree`] does is summarizing the metrics without
/// actually building the tree.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RootTree {
    mode: Mode,
    root: Box<Node>,
}

/// The trait to be implemented for a dhat prefix tree
pub trait Tree {
    /// Create a new `Tree` from the given parameters
    fn from_json(dhat_data: DhatData, entry_point: &EntryPoint, frames: &[String]) -> Self
    where
        Self: std::marker::Sized + Default,
    {
        let mut globs = frames.iter().collect::<Vec<_>>();
        let glob = match entry_point {
            EntryPoint::None => None,
            EntryPoint::Default => Some(DEFAULT_TOGGLE.into()),
            EntryPoint::Custom(custom) => Some(custom.into()),
        };

        if let Some(glob) = &glob {
            globs.push(glob);
        }

        let mut indices = vec![];
        if !globs.is_empty() {
            for (index, frame) in dhat_data.frame_table.iter().enumerate() {
                if let Frame::Leaf(_, func_name, _) = frame {
                    for glob in &globs {
                        if glob.as_str().dowild(func_name) {
                            indices.push(index);
                        }
                    }
                }
            }
        }

        let mut tree = Self::default();
        tree.set_mode(dhat_data.mode);

        // This is the default behaviour
        if *entry_point == EntryPoint::None && frames.is_empty() {
            tree.insert_iter(dhat_data.program_points.into_iter());
        // Indices can only be present if there is a match of the entry point or the frames
        } else if !indices.is_empty() {
            tree.insert_iter(
                dhat_data
                    .program_points
                    .into_iter()
                    .filter(|p| p.frames.iter().any(|f| indices.contains(f))),
            );
        } else {
            // If there was an entry point or frames configured but didn't match any indices, do
            // nothing
            tree.set_root_data(Data::zero());
        }

        tree
    }

    /// Return the [`Data`] of the root
    fn get_root_data(&self) -> &Data;

    /// Insert a prefix with the given [`Data`] into this [`Tree`]
    fn insert(&mut self, prefix: &[usize], data: &Data);

    /// Insert all [`ProgramPoint`]s into this [`Tree`]
    fn insert_iter(&mut self, iter: impl Iterator<Item = ProgramPoint>) {
        for elem in iter {
            let data = Data::from(&elem);
            self.insert(&elem.frames, &data);
        }
    }

    /// Return the metrics of the root node
    fn metrics(&self) -> ToolMetrics {
        self.get_root_data().metrics(self.mode())
    }

    /// Return the dhat invocation [`Mode`]
    fn mode(&self) -> Mode;

    /// Set the dhat invocation [`Mode`]
    fn set_mode(&mut self, mode: Mode);

    /// Set the [`Data`] of the root
    fn set_root_data(&mut self, data: Data);
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

    fn metrics(&self, mode: Mode) -> ToolMetrics {
        // This is the same order as order of metrics in the log file output
        let metrics = match mode {
            Mode::Heap | Mode::Copy => [
                (DhatMetric::TotalBytes, Some(self.total_bytes)),
                (DhatMetric::TotalBlocks, Some(self.total_blocks)),
                // These should all be None in copy mode
                (DhatMetric::AtTGmaxBytes, self.bytes_at_max),
                (DhatMetric::AtTGmaxBlocks, self.blocks_at_max),
                (DhatMetric::AtTEndBytes, self.bytes_at_end),
                (DhatMetric::AtTEndBlocks, self.blocks_at_end),
                (DhatMetric::ReadsBytes, self.blocks_read),
                (DhatMetric::WritesBytes, self.blocks_write),
                (
                    DhatMetric::TotalLifetimes,
                    #[allow(clippy::cast_possible_truncation)]
                    self.total_lifetimes.map(|a| a as u64),
                ),
                (DhatMetric::MaximumBytes, self.maximum_bytes),
                (DhatMetric::MaximumBlocks, self.maximum_blocks),
            ],
            Mode::AdHoc => [
                (DhatMetric::TotalUnits, Some(self.total_bytes)),
                (DhatMetric::TotalEvents, Some(self.total_blocks)),
                // These should all be None in ad-hoc mode
                (DhatMetric::AtTGmaxBytes, self.bytes_at_max),
                (DhatMetric::AtTGmaxBlocks, self.blocks_at_max),
                (DhatMetric::AtTEndBytes, self.bytes_at_end),
                (DhatMetric::AtTEndBlocks, self.blocks_at_end),
                (DhatMetric::ReadsBytes, self.blocks_read),
                (DhatMetric::WritesBytes, self.blocks_write),
                (
                    DhatMetric::TotalLifetimes,
                    #[allow(clippy::cast_possible_truncation)]
                    self.total_lifetimes.map(|a| a as u64),
                ),
                (DhatMetric::MaximumBytes, self.maximum_bytes),
                (DhatMetric::MaximumBlocks, self.maximum_blocks),
            ],
        };

        let mut tool_metrics = Metrics::empty();
        for (key, value) in metrics
            .iter()
            .filter_map(|(metric, value)| value.map(|value| (metric, value)))
        {
            tool_metrics.insert(*key, value.into());
        }
        ToolMetrics::Dhat(tool_metrics)
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

impl From<DhatData> for DhatTree {
    fn from(value: DhatData) -> Self {
        let mut tree = Self::default();
        for pps in value.program_points {
            let data = Data::from(&pps);
            tree.insert(&pps.frames, &data);
        }

        tree
    }
}

impl Tree for DhatTree {
    /// Insert a prefix with the given [`Data`] into this tree
    ///
    /// The rust borrow checker without the polonius crate below would give a false positive.
    fn insert(&mut self, prefix: &[usize], data: &Data) {
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

    fn set_root_data(&mut self, data: Data) {
        self.root.data = data;
    }

    fn get_root_data(&self) -> &Data {
        &self.root.data
    }

    fn mode(&self) -> Mode {
        self.mode
    }

    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }
}

impl Node {
    /// Create a new `Node`
    pub fn new(prefix: Vec<usize>, children: Vec<Self>, data: Data) -> Self {
        Self {
            children,
            data,
            prefix,
        }
    }

    /// Create a new default `Node` with the given prefix
    pub fn with_prefix(prefix: Vec<usize>) -> Self {
        Self {
            prefix,
            children: Vec::default(),
            data: Data::default(),
        }
    }

    fn add_child(&mut self, prefix: &[usize], data: &Data) {
        self.children
            .push(Self::new(prefix.to_vec(), vec![], data.clone()));
    }

    fn find_child(&mut self, num: usize) -> Option<&mut Self> {
        self.children
            .iter_mut()
            .find(|node| node.prefix.first().is_some_and(|a| *a == num))
    }

    fn split(&mut self, index: usize, data: &Data) {
        let node = Self::new(
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

impl Tree for RootTree {
    fn insert(&mut self, _prefix: &[usize], data: &Data) {
        self.root.data.add(data);
    }

    fn set_root_data(&mut self, data: Data) {
        self.root.data = data;
    }

    fn get_root_data(&self) -> &Data {
        &self.root.data
    }

    fn mode(&self) -> Mode {
        self.mode
    }

    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
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

    fn data_fixture(num: u64) -> Data {
        Data {
            total_bytes: num,
            ..Default::default()
        }
    }

    fn dhat_tree_fixture() -> DhatTree {
        DhatTree {
            mode: Mode::Heap,
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

    #[test]
    fn test_dhat_tree_insert_empty() {
        let mut expected = DhatTree::default();
        expected.root.data = data_fixture(1);

        let mut tree = DhatTree::default();
        tree.insert(&[], &data_fixture(1));

        assert_eq!(tree, expected);
    }

    #[test]
    fn test_dhat_tree_insert_equal() {
        let expected = DhatTree {
            mode: Mode::Heap,
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

        let mut tree = dhat_tree_fixture();
        tree.insert(&[1, 2, 3], &data_fixture(1));

        assert_eq!(tree, expected);
    }

    #[test]
    fn test_dhat_tree_insert_full_longer() {
        let expected = DhatTree {
            mode: Mode::Heap,
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

        let mut tree = dhat_tree_fixture();
        tree.insert(&[1, 2, 3, 6], &data_fixture(1));

        assert_eq!(tree, expected);
    }

    #[test]
    fn test_dhat_tree_insert_full_shorter() {
        let expected = DhatTree {
            mode: Mode::Heap,
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

        let mut tree = dhat_tree_fixture();
        tree.insert(&[1], &data_fixture(1));

        assert_eq!(tree, expected);
    }

    #[test]
    fn test_dhat_tree_insert_mixed() {
        let expected = DhatTree {
            mode: Mode::Heap,
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

        let mut tree = dhat_tree_fixture();
        tree.insert(&[1, 6], &data_fixture(1));

        assert_eq!(tree, expected);
    }

    #[test]
    fn test_dhat_tree_insert_no_match() {
        let expected = DhatTree {
            mode: Mode::Heap,
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

        let mut tree = dhat_tree_fixture();
        tree.insert(&[6], &data_fixture(1));

        assert_eq!(tree, expected);
    }

    #[test]
    fn test_root_tree_insert() {
        let expected = RootTree {
            mode: Mode::Heap,
            root: Box::new(Node::new(vec![], vec![], data_fixture(1))),
        };

        let mut tree = RootTree::default();
        tree.insert(&[1, 2, 3], &data_fixture(1));

        assert_eq!(tree, expected);
    }

    #[test]
    fn test_root_tree_insert_two() {
        let expected = RootTree {
            mode: Mode::Heap,
            root: Box::new(Node::new(vec![], vec![], data_fixture(3))),
        };

        let mut tree = RootTree::default();
        tree.insert(&[1, 2, 3], &data_fixture(1));
        tree.insert(&[1], &data_fixture(2));

        assert_eq!(tree, expected);
    }
}
