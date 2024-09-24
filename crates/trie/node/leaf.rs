use crate::{
    error::TrieError,
    nibble::NibbleSlice,
    node::BranchNode,
    node_hash::{NodeEncoder, NodeHash, PathKind},
    state::TrieState,
    PathRLP, ValueRLP,
};

use super::{ExtensionNode, Node};
/// Leaf Node of an an Ethereum Compatible Patricia Merkle Trie
/// Contains the node's hash, value & path
#[derive(Debug, Clone, Default)]
pub struct LeafNode {
    pub path: PathRLP,
    pub value: ValueRLP,
}

impl LeafNode {
    /// Creates a new leaf node and stores the given (path, value) pair
    pub fn new(path: PathRLP, value: ValueRLP) -> Self {
        Self { path, value }
    }

    /// Returns the stored value if the given path matches the stored path
    pub fn get(&self, path: NibbleSlice) -> Result<Option<ValueRLP>, TrieError> {
        if path.cmp_rest(&self.path) {
            Ok(Some(self.value.clone()))
        } else {
            Ok(None)
        }
    }

    /// Stores the received value and returns the new root of the subtrie previously consisting of self
    pub fn insert(
        mut self,
        state: &mut TrieState,
        path: NibbleSlice,
        value: ValueRLP,
    ) -> Result<Node, TrieError> {
        /* Possible flow paths:
            Leaf { SelfPath, SelfValue } -> Leaf { SelfPath, Value }
            Leaf { SelfPath, SelfValue } -> Extension { Branch { [Self,...] Path, Value } }
            Leaf { SelfPath, SelfValue } -> Extension { Branch { [ Leaf { Path, Value } , ... ], SelfPath, SelfValue} }
            Leaf { SelfPath, SelfValue } -> Branch { [ Leaf { Path, Value }, Self, ... ], None, None}
        */
        // If the path matches the stored path, update the value and return self
        if path.cmp_rest(&self.path) {
            self.value = value;
            Ok(self.into())
        } else {
            let offset = path.count_prefix_slice(&{
                let mut value_path = NibbleSlice::new(&self.path);
                value_path.offset_add(path.offset());
                value_path
            });

            let mut path_branch = path.clone();
            path_branch.offset_add(offset);

            let absolute_offset = path_branch.offset();
            // The offset that will be used when computing the hash of newly created leaf nodes
            let leaf_offset = absolute_offset + 1;
            let branch_node = if absolute_offset == 2 * path.as_ref().len() {
                // Create a branch node with self as a child and store the value in the branch node
                // Branch { [Self,...] Path, Value }
                let mut choices = BranchNode::EMPTY_CHOICES;
                choices[NibbleSlice::new(self.path.as_ref())
                    .nth(absolute_offset)
                    .unwrap() as usize] = self.clone().insert_self(leaf_offset, state)?;

                BranchNode::new_with_value(Box::new(choices), path.data(), value)
            } else if absolute_offset == 2 * self.path.len() {
                // Create a new leaf node and store the path and value in it
                // Create a new branch node with the leaf as a child and store self's path and value
                // Branch { [ Leaf { Path, Value } , ... ], SelfPath, SelfValue}
                let new_leaf = LeafNode::new(path.data(), value);
                let mut choices = BranchNode::EMPTY_CHOICES;
                choices[path_branch.next().unwrap() as usize] =
                    new_leaf.insert_self(leaf_offset, state)?;

                BranchNode::new_with_value(Box::new(choices), self.path, self.value)
            } else {
                // Create a new leaf node and store the path and value in it
                // Create a new branch node with the leaf and self as children
                // Branch { [ Leaf { Path, Value }, Self, ... ], None, None}
                let new_leaf = LeafNode::new(path.data(), value);
                let child_hash = new_leaf.insert_self(leaf_offset, state)?;
                let mut choices = BranchNode::EMPTY_CHOICES;
                choices[NibbleSlice::new(self.path.as_ref())
                    .nth(absolute_offset)
                    .unwrap() as usize] = self.clone().insert_self(leaf_offset, state)?;
                choices[path_branch.next().unwrap() as usize] = child_hash;
                BranchNode::new(Box::new(choices))
            };

            let final_node = if offset != 0 {
                // Create an extension node with the branch node as child
                // Extension { BranchNode }
                let branch_hash = branch_node.insert_self(state)?;
                ExtensionNode::new(path.split_to_vec(offset), branch_hash).into()
            } else {
                branch_node.into()
            };

            Ok(final_node)
        }
    }

    /// Removes own value if the path matches own path and returns self and the value if it was removed
    pub fn remove(self, path: NibbleSlice) -> Result<(Option<Node>, Option<ValueRLP>), TrieError> {
        Ok(if path.cmp_rest(&self.path) {
            (None, Some(self.value))
        } else {
            (Some(self.into()), None)
        })
    }

    /// Computes the node's hash given the offset in the path traversed before reaching this node
    pub fn compute_hash(&self, offset: usize) -> NodeHash {
        NodeHash::from_encoded_raw(self.encode_raw(offset))
    }

    /// Encodes the node given the offset in the path traversed before reaching this node
    pub fn encode_raw(&self, offset: usize) -> Vec<u8> {
        let encoded_value = &self.value;
        let encoded_path = &self.path;

        let mut path = NibbleSlice::new(encoded_path);
        path.offset_add(offset);

        let path_len = NodeEncoder::path_len(path.len());
        let value_len = NodeEncoder::bytes_len(
            encoded_value.len(),
            encoded_value.first().copied().unwrap_or_default(),
        );

        let mut encoder = crate::node_hash::NodeEncoder::new();
        encoder.write_list_header(path_len + value_len);
        encoder.write_path_slice(&path, PathKind::Leaf);
        encoder.write_bytes(encoded_value);
        encoder.finalize()
    }

    /// Inserts the node into the state and returns its hash
    /// Receives the offset that needs to be traversed to reach the leaf node from the canonical root, used to compute the node hash
    pub fn insert_self(
        self,
        path_offset: usize,
        state: &mut TrieState,
    ) -> Result<NodeHash, TrieError> {
        let hash = self.compute_hash(path_offset);
        state.insert_node(self.into(), hash.clone());
        Ok(hash)
    }

    /// Encodes the node and appends it to `node_path` if the encoded node is 32 or more bytes long
    pub fn get_path(
        &self,
        path: NibbleSlice,
        node_path: &mut Vec<Vec<u8>>,
    ) -> Result<(), TrieError> {
        let encoded = self.encode_raw(path.offset());
        if encoded.len() >= 32 {
            node_path.push(encoded);
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{pmt_node, Trie};

    #[test]
    fn new() {
        let node = LeafNode::new(Default::default(), Default::default());
        assert_eq!(node.path, PathRLP::default());
        assert_eq!(node.value, PathRLP::default());
    }

    #[test]
    fn get_some() {
        let node = pmt_node! { @(trie)
            leaf { vec![0x12] => vec![0x12, 0x34, 0x56, 0x78] }
        };

        assert_eq!(
            node.get(NibbleSlice::new(&[0x12])).unwrap(),
            Some(vec![0x12, 0x34, 0x56, 0x78]),
        );
    }

    #[test]
    fn get_none() {
        let node = pmt_node! { @(trie)
            leaf { vec![0x12] => vec![0x12, 0x34, 0x56, 0x78] }
        };

        assert!(node.get(NibbleSlice::new(&[0x34])).unwrap().is_none());
    }

    #[test]
    fn insert_replace() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            leaf { vec![0x12] => vec![0x12, 0x34, 0x56, 0x78] }
        };

        let node = node
            .insert(&mut trie.state, NibbleSlice::new(&[0x12]), vec![0x13])
            .unwrap();
        let node = match node {
            Node::Leaf(x) => x,
            _ => panic!("expected a leaf node"),
        };

        assert_eq!(node.path, vec![0x12]);
        assert_eq!(node.value, vec![0x13]);
    }

    #[test]
    fn insert_branch() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            leaf { vec![0x12] => vec![0x12, 0x34, 0x56, 0x78] }
        };
        let path = NibbleSlice::new(&[0x22]);
        let value = vec![0x23];
        let node = node
            .insert(&mut trie.state, path.clone(), value.clone())
            .unwrap();
        let node = match node {
            Node::Branch(x) => x,
            _ => panic!("expected a branch node"),
        };
        assert_eq!(node.get(&trie.state, path).unwrap(), Some(value));
    }

    #[test]
    fn insert_extension_branch() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            leaf { vec![0x12] => vec![0x12, 0x34, 0x56, 0x78] }
        };

        let path = NibbleSlice::new(&[0x13]);
        let value = vec![0x15];

        let node = node
            .insert(&mut trie.state, path.clone(), value.clone())
            .unwrap();

        assert!(matches!(node, Node::Extension(_)));
        assert_eq!(node.get(&trie.state, path).unwrap(), Some(value));
    }

    #[test]
    fn insert_extension_branch_value_self() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            leaf { vec![0x12] => vec![0x12, 0x34, 0x56, 0x78] }
        };

        let path = NibbleSlice::new(&[0x12, 0x34]);
        let value = vec![0x17];

        let node = node
            .insert(&mut trie.state, path.clone(), value.clone())
            .unwrap();

        assert!(matches!(node, Node::Extension(_)));
        assert_eq!(node.get(&trie.state, path).unwrap(), Some(value));
    }

    #[test]
    fn insert_extension_branch_value_other() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            leaf { vec![0x12, 0x34] => vec![0x12, 0x34, 0x56, 0x78] }
        };

        let path = NibbleSlice::new(&[0x12]);
        let value = vec![0x17];

        let node = node
            .insert(&mut trie.state, path.clone(), value.clone())
            .unwrap();

        assert!(matches!(node, Node::Extension(_)));
        assert_eq!(node.get(&trie.state, path).unwrap(), Some(value));
    }

    // An insertion that returns branch [value=(x)] -> leaf (y) is not possible because of the path
    // restrictions: nibbles come in pairs. If the first nibble is different, the node will be a
    // branch but it cannot have a value. If the second nibble is different, then it'll be an
    // extension followed by a branch with value and a child.

    // Because of that, the two tests that would check those cases are neither necessary nor
    // possible.

    #[test]
    fn remove_self() {
        let node = LeafNode::new(vec![0x12, 0x34], vec![0x12, 0x34, 0x56, 0x78]);
        let (node, value) = node.remove(NibbleSlice::new(&[0x12, 0x34])).unwrap();

        assert!(node.is_none());
        assert_eq!(value, Some(vec![0x12, 0x34, 0x56, 0x78]));
    }

    #[test]
    fn remove_none() {
        let node = LeafNode::new(vec![0x12, 0x34], vec![0x12, 0x34, 0x56, 0x78]);

        let (node, value) = node.remove(NibbleSlice::new(&[0x12])).unwrap();

        assert!(node.is_some());
        assert_eq!(value, None);
    }

    #[test]
    fn compute_hash() {
        let node = LeafNode::new(b"key".to_vec(), b"value".to_vec());
        let node_hash_ref = node.compute_hash(0);
        assert_eq!(
            node_hash_ref.as_ref(),
            &[0xCB, 0x84, 0x20, 0x6B, 0x65, 0x79, 0x85, 0x76, 0x61, 0x6C, 0x75, 0x65],
        );
    }

    #[test]
    fn compute_hash_long() {
        let node = LeafNode::new(b"key".to_vec(), b"a comparatively long value".to_vec());

        let node_hash_ref = node.compute_hash(0);
        assert_eq!(
            node_hash_ref.as_ref(),
            &[
                0xEB, 0x92, 0x75, 0xB3, 0xAE, 0x09, 0x3A, 0x17, 0x75, 0x7C, 0xFB, 0x42, 0xF7, 0xD5,
                0x57, 0xF9, 0xE5, 0x77, 0xBD, 0x5B, 0xEB, 0x86, 0xA8, 0x68, 0x49, 0x91, 0xA6, 0x5B,
                0x87, 0x5F, 0x80, 0x7A,
            ],
        );
    }
}
