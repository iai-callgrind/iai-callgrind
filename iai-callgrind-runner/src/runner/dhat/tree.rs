use std::cmp::Ordering;
use std::ops::Add;

use polonius_the_crab::{polonius, ForLt, PoloniusResult};

use super::model::ProgramPoint;

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
    prefix: Vec<u64>,
    children: Vec<Node>,
    data: Data,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Tree {
    root: Box<Node>,
}

impl Data {
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

impl From<ProgramPoint> for Data {
    fn from(value: ProgramPoint) -> Self {
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
    pub fn new(prefix: Vec<u64>, children: Vec<Node>, data: Data) -> Self {
        Self {
            prefix,
            children,
            data,
        }
    }

    pub fn with_prefix(prefix: Vec<u64>) -> Self {
        Self {
            prefix,
            children: Vec::default(),
            data: Data::default(),
        }
    }

    fn add_child(&mut self, prefix: &[u64], data: &Data) {
        self.children
            .push(Node::new(prefix.to_vec(), vec![], data.clone()));
    }

    fn find_child(&mut self, num: u64) -> Option<&mut Self> {
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

    fn split_index(&self, other: &[u64]) -> Option<usize> {
        let length = self.prefix.len().min(other.len());
        (0..length).find(|&index| self.prefix[index] != other[index])
    }

    fn add_data(&mut self, data: &Data) {
        self.data.add(data);
    }
}

impl Tree {
    pub fn insert(&mut self, prefix: &[u64], data: &Data) {
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
