use crate::error::TrieError;
use crate::nibble::NibbleSlice;
use crate::nibble::NibbleVec;
use crate::node_hash::{NodeEncoder, NodeHash, PathKind};
use crate::state::TrieState;
use crate::ValueRLP;

use super::{BranchNode, LeafNode, Node};

/// Extension Node of an an Ethereum Compatible Patricia Merkle Trie
/// Contains the node's prefix and a its child node hash, doesn't store any value
#[derive(Debug, Clone)]
pub struct ExtensionNode {
    pub prefix: NibbleVec,
    pub child: NodeHash,
}

impl ExtensionNode {
    /// Creates a new extension node given its child hash and prefix
    pub(crate) fn new(prefix: NibbleVec, child: NodeHash) -> Self {
        Self { prefix, child }
    }

    /// Retrieves a value from the subtrie originating from this node given its path
    pub fn get(
        &self,
        state: &TrieState,
        mut path: NibbleSlice,
    ) -> Result<Option<ValueRLP>, TrieError> {
        // If the path is prefixed by this node's prefix, delegate to its child.
        // Otherwise, no value is present.
        if path.skip_prefix(&self.prefix) {
            let child_node = state
                .get_node(self.child.clone())?
                .expect("inconsistent internal tree structure");

            child_node.get(state, path)
        } else {
            Ok(None)
        }
    }

    /// Inserts a value into the subtrie originating from this node and returns the new root of the subtrie
    pub fn insert(
        mut self,
        state: &mut TrieState,
        mut path: NibbleSlice,
        value: ValueRLP,
    ) -> Result<Node, TrieError> {
        /* Possible flow paths (there are duplicates between different prefix lengths):
            Extension { prefix, child } -> Extension { prefix , child' } (insert into child)
            Extension { prefixL+C+prefixR, child } -> Extension { prefixL, Branch { [ Extension { prefixR, child }, ..], Path, Value} } (if path fully traversed)
            Extension { prefixL+C+prefixR, child } -> Extension { prefixL, Branch { [ Extension { prefixR, child }, Leaf { Path, Value }..] None, None} } (if path not fully traversed)
            Extension { prefixL+C+None, child } -> Extension { prefixL, Branch { [child, ... ], Path, Value} } (if path fully traversed)
            Extension { prefixL+C+None, child } -> Extension { prefixL, Branch { [child, ... ], Leaf { Path, Value }, ... }, None, None } (if path not fully traversed)
            Extension { None+C+prefixR } -> Branch { [ Extension { prefixR, child } , ..], Path, Value} (if path fully traversed)
            Extension { None+C+prefixR } -> Branch { [ Extension { prefixR, child } , Leaf { Path, Value } , ... ], None, None} (if path not fully traversed)
        */

        if path.skip_prefix(&self.prefix) {
            // Insert into child node
            let child_node = state
                .get_node(self.child)?
                .expect("inconsistent internal tree structure");
            let child_node = child_node.insert(state, path.clone(), value.clone())?;
            // Child node will never be a leaf, so the path_offset is not used
            self.child = child_node.insert_self(0, state)?;

            Ok(self.into())
        } else {
            let offset = path.clone().count_prefix_vec(&self.prefix);
            path.offset_add(offset);
            // Offset used when computing the hash of the new child
            let child_offset = path.offset() + 1;
            // Split prefix into left_prefix and right_prefix
            let (left_prefix, choice, right_prefix) = self.prefix.split_extract_at(offset);

            let left_prefix = (!left_prefix.is_empty()).then_some(left_prefix);
            let right_prefix = (!right_prefix.is_empty()).then_some(right_prefix);

            // Create right prefix node:
            // If there is a right prefix the node will be Extension { prefixR, Self.child }
            // If there is no right prefix the node will be Self.child
            let right_prefix_node = if let Some(right_prefix) = right_prefix {
                let extension_node = ExtensionNode::new(right_prefix, self.child);
                extension_node.insert_self(state)?
            } else {
                self.child
            };

            // Create a branch node:
            // If the path hasn't been traversed: Branch { [ RPrefixNode, Leaf { Path, Value }, ... ], None, None }
            // If the path has been fully traversed: Branch { [ RPrefixNode, ... ], Path, Value }
            let mut choices = BranchNode::EMPTY_CHOICES;
            choices[choice as usize] = right_prefix_node;
            let branch_node = if let Some(c) = path.next() {
                let new_leaf = LeafNode::new(path.data(), value);
                choices[c as usize] = new_leaf.insert_self(child_offset, state)?;
                BranchNode::new(Box::new(choices))
            } else {
                BranchNode::new_with_value(Box::new(choices), path.data(), value)
            };

            // Create a final node, then return it:
            // If there is a left_prefix: Extension { left_prefix , Branch }
            // If there is no left_prefix: Branch
            match left_prefix {
                Some(left_prefix) => {
                    let branch_hash = branch_node.insert_self(state)?;
                    Ok(ExtensionNode::new(left_prefix, branch_hash).into())
                }
                None => Ok(branch_node.into()),
            }
        }
    }

    pub fn remove(
        mut self,
        state: &mut TrieState,
        mut path: NibbleSlice,
    ) -> Result<(Option<Node>, Option<ValueRLP>), TrieError> {
        /* Possible flow paths:
            Extension { prefix, child } -> Extension { prefix, child } (no removal)
            Extension { prefix, child } -> None (If child.remove = None)
            Extension { prefix, child } -> Extension { prefix, ChildBranch } (if child.remove = Branch)
            Extension { prefix, child } -> ChildExtension { SelfPrefix+ChildPrefix, ChildExtensionChild } (if child.remove = Extension)
            Extension { prefix, child } -> ChildLeaf (if child.remove = Leaf)
        */

        // Check if the value is part of the child subtrie according to the prefix
        if path.skip_prefix(&self.prefix) {
            let child_node = state
                .get_node(self.child)?
                .expect("inconsistent internal tree structure");
            // Remove value from child subtrie
            let (child_node, old_value) = child_node.remove(state, path.clone())?;
            // Restructure node based on removal
            let node = match child_node {
                // If there is no subtrie remove the node
                None => None,
                Some(node) => Some(match node {
                    // If it is a branch node set it as self's child
                    Node::Branch(branch_node) => {
                        self.child = branch_node.insert_self(state)?;
                        self.into()
                    }
                    // If it is an extension replace self with it after updating its prefix
                    Node::Extension(mut extension_node) => {
                        self.prefix.extend(&extension_node.prefix);
                        extension_node.prefix = self.prefix;
                        extension_node.into()
                    }
                    // If it is a leaf node replace self with it
                    Node::Leaf(leaf_node) => leaf_node.into(),
                }),
            };

            Ok((node, old_value))
        } else {
            Ok((Some(self.into()), None))
        }
    }

    /// Computes the node's hash
    pub fn compute_hash(&self) -> NodeHash {
        NodeHash::from_encoded_raw(self.encode_raw())
    }

    /// Encodes the node
    pub fn encode_raw(&self) -> Vec<u8> {
        let child_hash = &self.child;
        let prefix_len = NodeEncoder::path_len(self.prefix.len());
        let child_len = match child_hash {
            NodeHash::Inline(ref x) => x.len(),
            NodeHash::Hashed(x) => NodeEncoder::bytes_len(32, x[0]),
        };

        let mut encoder = NodeEncoder::new();
        encoder.write_list_header(prefix_len + child_len);
        encoder.write_path_vec(&self.prefix, PathKind::Extension);
        match child_hash {
            NodeHash::Inline(x) => encoder.write_raw(x),
            NodeHash::Hashed(x) => encoder.write_bytes(&x.0),
        }
        encoder.finalize()
    }

    /// Inserts the node into the state and returns its hash
    pub fn insert_self(self, state: &mut TrieState) -> Result<NodeHash, TrieError> {
        let hash = self.compute_hash();
        state.insert_node(self.into(), hash.clone());
        Ok(hash)
    }

    /// Traverses own subtrie until reaching the node containing `path`
    /// Appends all encoded nodes traversed to `node_path` (including self)
    /// Only nodes with encoded len over or equal to 32 bytes are included
    pub fn get_path(
        &self,
        state: &TrieState,
        mut path: NibbleSlice,
        node_path: &mut Vec<Vec<u8>>,
    ) -> Result<(), TrieError> {
        // Add self to node_path (if not inlined in parent)
        let encoded = self.encode_raw();
        if encoded.len() >= 32 {
            node_path.push(encoded);
        };
        // Continue to child
        if path.skip_prefix(&self.prefix) {
            let child_node = state
                .get_node(self.child.clone())?
                .expect("inconsistent internal tree structure");
            child_node.get_path(state, path, node_path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        pmt_node,
        {nibble::Nibble, Trie},
    };

    #[test]
    fn new() {
        let node = ExtensionNode::new(NibbleVec::new(), Default::default());

        assert_eq!(node.prefix.len(), 0);
        assert_eq!(node.child, Default::default());
    }

    #[test]
    fn get_some() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            extension { [0], branch {
                0 => leaf { vec![0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x01] => vec![0x34, 0x56, 0x78, 0x9A] },
            } }
        };

        assert_eq!(
            node.get(&trie.state, NibbleSlice::new(&[0x00])).unwrap(),
            Some(vec![0x12, 0x34, 0x56, 0x78]),
        );
        assert_eq!(
            node.get(&trie.state, NibbleSlice::new(&[0x01])).unwrap(),
            Some(vec![0x34, 0x56, 0x78, 0x9A]),
        );
    }

    #[test]
    fn get_none() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            extension { [0], branch {
                0 => leaf { vec![0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x01] => vec![0x34, 0x56, 0x78, 0x9A] },
            } }
        };

        assert_eq!(
            node.get(&trie.state, NibbleSlice::new(&[0x02])).unwrap(),
            None,
        );
    }

    #[test]
    fn insert_passthrough() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            extension { [0], branch {
                0 => leaf { vec![0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x01] => vec![0x34, 0x56, 0x78, 0x9A] },
            } }
        };

        let node = node
            .insert(&mut trie.state, NibbleSlice::new(&[0x02]), vec![])
            .unwrap();
        let node = match node {
            Node::Extension(x) => x,
            _ => panic!("expected an extension node"),
        };
        assert!(node.prefix.iter().eq([Nibble::V0].into_iter()));
    }

    #[test]
    fn insert_branch() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            extension { [0], branch {
                0 => leaf { vec![0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x01] => vec![0x34, 0x56, 0x78, 0x9A] },
            } }
        };

        let node = node
            .insert(&mut trie.state, NibbleSlice::new(&[0x10]), vec![0x20])
            .unwrap();
        let node = match node {
            Node::Branch(x) => x,
            _ => panic!("expected a branch node"),
        };
        assert_eq!(
            node.get(&trie.state, NibbleSlice::new(&[0x10])).unwrap(),
            Some(vec![0x20])
        );
    }

    #[test]
    fn insert_branch_extension() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            extension { [0, 0], branch {
                0 => leaf { vec![0x00, 0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x00, 0x10] => vec![0x34, 0x56, 0x78, 0x9A] },
            } }
        };

        let node = node
            .insert(&mut trie.state, NibbleSlice::new(&[0x10]), vec![0x20])
            .unwrap();
        let node = match node {
            Node::Branch(x) => x,
            _ => panic!("expected a branch node"),
        };
        assert_eq!(
            node.get(&trie.state, NibbleSlice::new(&[0x10])).unwrap(),
            Some(vec![0x20])
        );
    }

    #[test]
    fn insert_extension_branch() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            extension { [0, 0], branch {
                0 => leaf { vec![0x00, 0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x00, 0x10] => vec![0x34, 0x56, 0x78, 0x9A] },
            } }
        };

        let path = NibbleSlice::new(&[0x01]);
        let value = vec![0x02];

        let node = node
            .insert(&mut trie.state, path.clone(), value.clone())
            .unwrap();

        assert!(matches!(node, Node::Extension(_)));
        assert_eq!(node.get(&trie.state, path).unwrap(), Some(value));
    }

    #[test]
    fn insert_extension_branch_extension() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            extension { [0, 0], branch {
                0 => leaf { vec![0x00, 0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x00, 0x10] => vec![0x34, 0x56, 0x78, 0x9A] },
            } }
        };

        let path = NibbleSlice::new(&[0x01]);
        let value = vec![0x04];

        let node = node
            .insert(&mut trie.state, path.clone(), value.clone())
            .unwrap();

        assert!(matches!(node, Node::Extension(_)));
        assert_eq!(node.get(&trie.state, path).unwrap(), Some(value));
    }

    #[test]
    fn remove_none() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            extension { [0], branch {
                0 => leaf { vec![0x00] => vec![0x00] },
                1 => leaf { vec![0x01] => vec![0x01] },
            } }
        };

        let (node, value) = node
            .remove(&mut trie.state, NibbleSlice::new(&[0x02]))
            .unwrap();

        assert!(matches!(node, Some(Node::Extension(_))));
        assert_eq!(value, None);
    }

    #[test]
    fn remove_into_leaf() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            extension { [0], branch {
                0 => leaf { vec![0x00] => vec![0x00] },
                1 => leaf { vec![0x01] => vec![0x01] },
            } }
        };

        let (node, value) = node
            .remove(&mut trie.state, NibbleSlice::new(&[0x01]))
            .unwrap();

        assert!(matches!(node, Some(Node::Leaf(_))));
        assert_eq!(value, Some(vec![0x01]));
    }

    #[test]
    fn remove_into_extension() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            extension { [0], branch {
                0 => leaf { vec![0x00] => vec![0x00] },
                1 => extension { [0], branch {
                    0 => leaf { vec![0x01, 0x00] => vec![0x01, 0x00] },
                    1 => leaf { vec![0x01, 0x01] => vec![0x01, 0x01] },
                } },
            } }
        };

        let (node, value) = node
            .remove(&mut trie.state, NibbleSlice::new(&[0x00]))
            .unwrap();

        assert!(matches!(node, Some(Node::Extension(_))));
        assert_eq!(value, Some(vec![0x00]));
    }

    #[test]
    fn compute_hash() {
        /*
        Extension {
            [0x00, 0x00]
            Branch { [
                Leaf { [0x00, 0x00], [0x12, 0x34] }
                Leaf { [0x00, 0x10], [0x56, 0x78] }
            }
        }
        */
        let leaf_node_a = LeafNode::new(vec![0x00, 0x00], vec![0x12, 0x34]);
        let leaf_node_b = LeafNode::new(vec![0x00, 0x10], vec![0x56, 0x78]);
        let mut choices = BranchNode::EMPTY_CHOICES;
        choices[0] = leaf_node_a.compute_hash(3);
        choices[1] = leaf_node_b.compute_hash(3);
        let branch_node = BranchNode::new(Box::new(choices));
        let node = ExtensionNode::new(
            NibbleVec::from_nibbles([Nibble::V0, Nibble::V0].into_iter(), false),
            branch_node.compute_hash(),
        );

        assert_eq!(
            node.compute_hash().as_ref(),
            &[
                0xDD, 0x82, 0x00, 0x00, 0xD9, 0xC4, 0x30, 0x82, 0x12, 0x34, 0xC4, 0x30, 0x82, 0x56,
                0x78, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
                0x80, 0x80,
            ],
        );
    }

    #[test]
    fn compute_hash_long() {
        /*
        Extension {
            [0x00, 0x00]
            Branch { [
                Leaf { [0x00, 0x00], [0x12, 0x34, 0x56, 0x78, 0x9A] }
                Leaf { [0x00, 0x10], [0x34, 0x56, 0x78, 0x9A, 0xBC] }
            }
        }
        */
        let leaf_node_a = LeafNode::new(vec![0x00, 0x00], vec![0x12, 0x34, 0x56, 0x78, 0x9A]);
        let leaf_node_b = LeafNode::new(vec![0x00, 0x10], vec![0x34, 0x56, 0x78, 0x9A, 0xBC]);
        let mut choices = BranchNode::EMPTY_CHOICES;
        choices[0] = leaf_node_a.compute_hash(3);
        choices[1] = leaf_node_b.compute_hash(3);
        let branch_node = BranchNode::new(Box::new(choices));
        let node = ExtensionNode::new(
            NibbleVec::from_nibbles([Nibble::V0, Nibble::V0].into_iter(), false),
            branch_node.compute_hash(),
        );

        assert_eq!(
            node.compute_hash().as_ref(),
            &[
                0xFA, 0xBA, 0x42, 0x79, 0xB3, 0x9B, 0xCD, 0xEB, 0x7C, 0x53, 0x0F, 0xD7, 0x6E, 0x5A,
                0xA3, 0x48, 0xD3, 0x30, 0x76, 0x26, 0x14, 0x84, 0x55, 0xA0, 0xAE, 0xFE, 0x0F, 0x52,
                0x89, 0x5F, 0x36, 0x06,
            ],
        );
    }
}
