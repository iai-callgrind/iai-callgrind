// #[derive(Debug, Default)]
// struct Data {
//     total_bytes: u64,
// }

use std::cmp::Ordering;

use polonius_the_crab::{polonius, ForLt, PoloniusResult};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Node {
    prefix: Vec<u64>,
    children: Vec<Node>,
    // data: Data,
}

impl Node {
    fn new(prefix: Vec<u64>, children: Vec<Node>) -> Self {
        Self { prefix, children }
    }

    fn with_prefix(prefix: Vec<u64>) -> Self {
        Self {
            prefix,
            children: Vec::default(),
        }
    }

    fn add_child(&mut self, node: Node) {
        self.children.push(node);
    }

    fn find_child(&mut self, num: u64) -> Option<&mut Self> {
        self.children
            .iter_mut()
            .find(|node| node.prefix.first().is_some_and(|a| *a == num))
    }

    fn split(&mut self, index: usize) {
        let node = Node::new(
            self.prefix.split_off(index),
            std::mem::take(&mut self.children),
        );
        self.children.push(node);
    }

    fn split_index(&self, other: &[u64]) -> Option<usize> {
        let length = self.prefix.len().min(other.len());
        (0..length).find(|&index| self.prefix[index] != other[index])
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Tree {
    root: Box<Node>,
}

impl Tree {
    pub fn insert(&mut self, prefix: &[u64]) {
        let mut current = &mut *self.root;
        let mut index = 0;

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
                        child.split(split_index);
                        index += split_index;
                    } else {
                        match current_prefix.len().cmp(&child.prefix.len()) {
                            Ordering::Less => {
                                child.split(current_prefix.len());
                                return;
                            }
                            Ordering::Greater => {
                                index += child.prefix.len();
                            }
                            Ordering::Equal => {
                                // do nothing
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
                    current.add_child(Node::with_prefix(current_prefix.to_vec()));
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree_fixture() -> Tree {
        Tree {
            root: Box::new(Node::new(
                vec![],
                vec![Node::new(
                    vec![1, 2, 3],
                    vec![Node::new(vec![4, 5], vec![])],
                )],
            )),
        }
    }

    #[test]
    fn test_insert_empty() {
        let mut tree = Tree::default();
        tree.insert(&[]);

        assert_eq!(tree, Tree::default());
    }

    #[test]
    fn test_insert_full_shorter() {
        let expected = Tree {
            root: Box::new(Node::new(
                vec![],
                vec![Node::new(
                    vec![1],
                    vec![Node::new(vec![2, 3], vec![Node::new(vec![4, 5], vec![])])],
                )],
            )),
        };

        let mut tree = tree_fixture();
        tree.insert(&[1]);

        assert_eq!(tree, expected);
    }

    #[test]
    fn test_insert_full_longer() {
        let expected = Tree {
            root: Box::new(Node::new(
                vec![],
                vec![Node::new(
                    vec![1, 2, 3],
                    vec![Node::new(vec![4, 5], vec![]), Node::new(vec![6], vec![])],
                )],
            )),
        };

        let mut tree = tree_fixture();
        tree.insert(&[1, 2, 3, 6]);

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
                        Node::new(vec![2, 3], vec![Node::new(vec![4, 5], vec![])]),
                        Node::new(vec![6], vec![]),
                    ],
                )],
            )),
        };

        let mut tree = tree_fixture();
        tree.insert(&[1, 6]);

        assert_eq!(tree, expected);
    }

    #[test]
    fn test_insert_no_match() {
        let expected = Tree {
            root: Box::new(Node::new(
                vec![],
                vec![
                    Node::new(vec![1, 2, 3], vec![Node::with_prefix(vec![4, 5])]),
                    Node::with_prefix(vec![6]),
                ],
            )),
        };

        let mut tree = tree_fixture();
        tree.insert(&[6]);

        assert_eq!(tree, expected);
    }

    #[test]
    fn test_insert_equal() {
        let mut tree = tree_fixture();
        tree.insert(&[1, 2, 3]);

        assert_eq!(tree, tree_fixture());
    }
}
