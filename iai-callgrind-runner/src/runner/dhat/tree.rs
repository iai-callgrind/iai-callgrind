// #[derive(Debug, Default)]
// struct Data {
//     total_bytes: u64,
// }

use std::cmp::Ordering;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Node {
    prefix: Vec<u64>,
    nodes: Vec<Node>,
    // data: Data,
}

impl Node {
    fn prefix(&self) -> &[u64] {
        &self.prefix
    }

    fn new(prefix: Vec<u64>) -> Self {
        Self {
            prefix,
            ..Default::default()
        }
    }

    fn lookup(&self, other: &[u64]) -> Option<&Self> {
        match self.prefix.len().cmp(&other.len()) {
            Ordering::Less => {
                let new = if self.is_root() {
                    other
                } else {
                    other.split_at(self.prefix.len()).1
                };
                for node in &self.nodes {
                    if new.starts_with(&node.prefix) || node.prefix.starts_with(new) {
                        return node.lookup(new);
                    }
                }
                None
            }
            Ordering::Equal => Some(self),
            Ordering::Greater => None,
        }
    }

    /// Insert a new `Node` which doesn't have sub-nodes and of which is known that is a prefix of
    /// this `Node` or this `Node` is a prefix of the new `Node`.
    fn insert(&mut self, mut other: Node) {
        match self.prefix.len().cmp(&other.prefix.len()) {
            Ordering::Less => {
                if !self.is_root() {
                    other.prefix = other.prefix.split_off(self.prefix.len());
                };
                for node in &mut self.nodes {
                    if node.is_prefix(&other) || other.is_prefix(node) {
                        node.insert(other);
                        return;
                    }
                }
                self.nodes.push(other);
            }
            Ordering::Equal => {
                // do nothing, same node
            }
            // Insert [1, 2] in [1, 2, 3] -> [4],[5] => [1, 2] -> [3] -> [4],[5]
            Ordering::Greater => {
                // The `other` Node must be the new Node at this point which doesn't have sub-nodes
                assert!(other.nodes.is_empty());
                other.prefix = self.prefix.split_off(other.prefix.len());
                other.nodes = std::mem::take(&mut self.nodes);

                self.nodes.push(other);
            }
        }
    }

    fn is_root(&self) -> bool {
        self.prefix.is_empty()
    }

    #[inline]
    fn is_prefix(&self, other: &Node) -> bool {
        other.prefix.starts_with(&self.prefix)
    }
}

#[derive(Debug, Default)]
pub struct Tree {
    root: Node,
}

impl Tree {
    pub fn insert(&mut self, node: Node) {
        if node.prefix.is_empty() {
            return;
        }
        self.root.insert(node);
    }

    pub fn lookup_prefix(&self, prefix: &[u64]) -> Option<&Node> {
        if prefix.is_empty() {
            return None;
        }

        self.root.lookup(prefix)
    }

    pub fn insert_prefix(&mut self, prefix: Vec<u64>) {
        self.insert(Node::new(prefix));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev() {
        let mut tree = Tree::default();
        tree.insert(Node {
            prefix: vec![],
            ..Default::default()
        });
        assert!(tree.root.prefix.is_empty());
        assert!(tree.root.nodes.is_empty());

        let mut tree = Tree::default();
        tree.insert(Node {
            prefix: vec![1],
            ..Default::default()
        });
        tree.insert(Node {
            prefix: vec![1, 2, 3],
            ..Default::default()
        });
        tree.insert(Node {
            prefix: vec![1, 2, 3, 4],
            ..Default::default()
        });
        tree.insert(Node {
            prefix: vec![1, 2, 3, 5],
            ..Default::default()
        });
        tree.insert(Node {
            prefix: vec![1, 2],
            ..Default::default()
        });
        assert_eq!(
            tree.lookup_prefix(&[1, 2, 3]).map(super::Node::prefix),
            Some([3].as_slice())
        );
        // assert_eq!(tree.root.nodes.first().unwrap().prefix, vec![1, 2, 3]);
        // tree.insert(vec![1, 2, 3, 4]);
        // assert_eq!(
        //     tree.root
        //         .nodes
        //         .first()
        //         .unwrap()
        //         .nodes
        //         .first()
        //         .unwrap()
        //         .prefix,
        //     vec![4]
        // );
        // tree.insert(vec![1, 2, 3, 4, 5]);
        // tree.insert(vec![1, 2, 3, 5]);
        // tree.insert(vec![2, 3, 4]);
        // dbg!(&tree);
        // assert!(false);
        // assert_eq!(
        //     root.nodes.first().unwrap().nodes.first().unwrap().prefix,
        //     vec![4]
        // );

        // let mut root = Node::default();
        // root.insert(vec![1, 2, 3]);
        // assert_eq!(root.nodes.first().unwrap().prefix, vec![1, 2, 3]);
    }
}
