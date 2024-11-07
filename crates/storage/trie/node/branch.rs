use crate::{
    error::TrieError,
    nibble::{Nibble, NibbleSlice, NibbleVec},
    node_hash::{NodeEncoder, NodeHash},
    state::TrieState,
    PathRLP, ValueRLP,
};

use super::{ExtensionNode, LeafNode, Node};

/// Branch Node of an an Ethereum Compatible Patricia Merkle Trie
/// Contains the node's hash, value, path, and the hash of its children nodes
#[derive(Debug, Clone)]
pub struct BranchNode {
    // TODO: check if switching to hashmap is a better solution
    pub choices: Box<[NodeHash; 16]>,
    pub path: PathRLP,
    pub value: ValueRLP,
}

impl BranchNode {
    /// Empty choice array for more convenient node-building
    pub const EMPTY_CHOICES: [NodeHash; 16] = [
        NodeHash::const_default(),
        NodeHash::const_default(),
        NodeHash::const_default(),
        NodeHash::const_default(),
        NodeHash::const_default(),
        NodeHash::const_default(),
        NodeHash::const_default(),
        NodeHash::const_default(),
        NodeHash::const_default(),
        NodeHash::const_default(),
        NodeHash::const_default(),
        NodeHash::const_default(),
        NodeHash::const_default(),
        NodeHash::const_default(),
        NodeHash::const_default(),
        NodeHash::const_default(),
    ];

    /// Creates a new branch node given its children, without any stored value
    pub fn new(choices: Box<[NodeHash; 16]>) -> Self {
        Self {
            choices,
            path: Default::default(),
            value: Default::default(),
        }
    }

    /// Creates a new branch node given its children and stores the given (path, value) pair
    pub fn new_with_value(choices: Box<[NodeHash; 16]>, path: PathRLP, value: ValueRLP) -> Self {
        Self {
            choices,
            path,
            value,
        }
    }

    /// Updates the node's path and value
    pub fn update(&mut self, new_path: PathRLP, new_value: ValueRLP) {
        self.path = new_path;
        self.value = new_value;
    }

    /// Retrieves a value from the subtrie originating from this node given its path
    pub fn get(
        &self,
        state: &TrieState,
        mut path: NibbleSlice,
    ) -> Result<Option<ValueRLP>, TrieError> {
        // If path is at the end, return to its own value if present.
        // Otherwise, check the corresponding choice and delegate accordingly if present.
        if let Some(choice) = path.next().map(usize::from) {
            // Delegate to children if present
            let child_hash = &self.choices[choice];
            if child_hash.is_valid() {
                let child_node = state
                    .get_node(child_hash.clone())?
                    .expect("inconsistent internal tree structure");
                child_node.get(state, path)
            } else {
                Ok(None)
            }
        } else {
            // Return internal value if present.
            Ok((!self.value.is_empty()).then_some(self.value.clone()))
        }
    }

    /// Inserts a value into the subtrie originating from this node and returns the new root of the subtrie
    pub fn insert(
        mut self,
        state: &mut TrieState,
        mut path: NibbleSlice,
        value: ValueRLP,
    ) -> Result<Node, TrieError> {
        // If path is at the end, insert or replace its own value.
        // Otherwise, check the corresponding choice and insert or delegate accordingly.
        match path.next() {
            Some(choice) => match &mut self.choices[choice as usize] {
                // Create new child (leaf node)
                choice_hash if !choice_hash.is_valid() => {
                    let new_leaf = LeafNode::new(path.data(), value);
                    let child_hash = new_leaf.insert_self(path.offset(), state)?;
                    *choice_hash = child_hash;
                }
                // Insert into existing child and then update it
                choice_hash => {
                    let child_node = state
                        .get_node(choice_hash.clone())?
                        .expect("inconsistent internal tree structure");

                    let child_node = child_node.insert(state, path.clone(), value)?;
                    *choice_hash = child_node.insert_self(path.offset(), state)?;
                }
            },
            None => {
                // Insert into self
                self.update(path.data(), value);
            }
        };

        Ok(self.into())
    }

    /// Removes a value from the subtrie originating from this node given its path
    /// Returns the new root of the subtrie (if any) and the removed value if it existed in the subtrie
    pub fn remove(
        mut self,
        state: &mut TrieState,
        mut path: NibbleSlice,
    ) -> Result<(Option<Node>, Option<ValueRLP>), TrieError> {
        /* Possible flow paths:
            Step 1: Removal
                Branch { [ ... ], Path, Value } -> Branch { [...], None, None } (remove from self)
                Branch { [ childA, ... ], Path, Value } -> Branch { [childA', ... ], Path, Value } (remove from child)

            Step 2: Restructure
                [0 children]
                Branch { [], Path, Value } -> Leaf { Path, Value } (no children, with value)
                Branch { [], None, None } -> Branch { [], None, None } (no children, no value)
                [1 child]
                Branch { [ ExtensionChild], _ , _ } -> Extension { ChoiceIndex+ExtensionChildPrefx, ExtensionChildChild }
                Branch { [ BranchChild ], None, None } -> Extension { ChoiceIndex, BranchChild }
                Branch { [ LeafChild], None, None } -> LeafChild
                Branch { [LeafChild], Path, Value } -> Branch { [ LeafChild ], Path, Value }
                [+1 children]
                Branch { [childA, childB, ... ], None, None } ->   Branch { [childA, childB, ... ], None, None }
        */

        // Step 1: Remove value

        let path_offset = path.offset();
        // Check if the value is located in a child subtrie
        let value = match path.next() {
            Some(choice_index) => {
                if self.choices[choice_index as usize].is_valid() {
                    let child_node = state
                        .get_node(self.choices[choice_index as usize].clone())?
                        .expect("inconsistent internal tree structure");
                    // Remove value from child node
                    let (child_node, old_value) = child_node.remove(state, path.clone())?;
                    if let Some(child_node) = child_node {
                        // Update child node
                        self.choices[choice_index as usize] =
                            child_node.insert_self(path.offset(), state)?;
                    } else {
                        // Remove child hash if the child subtrie was removed in the process
                        self.choices[choice_index as usize] = NodeHash::default();
                    }
                    old_value
                } else {
                    None
                }
            }
            None => {
                // Remove own value (if it has one) and return it
                if !self.path.is_empty() {
                    let value = self.value;
                    self.path = Default::default();
                    self.value = Default::default();

                    (!value.is_empty()).then_some(value)
                } else {
                    None
                }
            }
        };

        // Step 2: Restructure self

        // Check if self only has one child left

        // An `Err(_)` means more than one choice. `Ok(Some(_))` and `Ok(None)` mean a single and no
        // choices respectively.
        // If there is only one child choice_count will contain the choice index and the hash of the child node
        let choice_count = self
            .choices
            .iter_mut()
            .enumerate()
            .try_fold(None, |acc, (i, x)| {
                Ok(match (acc, x.is_valid()) {
                    (None, true) => Some((i, x)),
                    (None, false) => None,
                    (Some(_), true) => return Err(()),
                    (Some((i, x)), false) => Some((i, x)),
                })
            });

        let child_hash = match choice_count {
            Ok(Some((choice_index, child_hash))) => {
                let choice_index = Nibble::try_from(choice_index as u8).unwrap();
                let child_node = state
                    .get_node(child_hash.clone())?
                    .expect("inconsistent internal tree structure");

                match child_node {
                    // Replace the child node  with an extension node leading to it
                    // The extension node will then replace self if self has no value
                    Node::Branch(_) => {
                        let extension_node = ExtensionNode::new(
                            NibbleVec::from_single(choice_index, path_offset % 2 != 0),
                            child_hash.clone(),
                        );
                        *child_hash = extension_node.insert_self(state)?
                    }
                    // Replace self with the child extension node, updating its path in the process
                    Node::Extension(mut extension_node) => {
                        debug_assert!(self.path.is_empty()); // Sanity check
                        extension_node.prefix.prepend(choice_index);
                        // Return node here so we don't have to update it in the state and then fetch it
                        return Ok((Some(extension_node.into()), value));
                    }
                    _ => {}
                }

                Some(child_hash)
            }
            _ => None,
        };

        let new_node = match (child_hash, !self.path.is_empty()) {
            // If this node still has a child and value return the updated node
            (Some(_), true) => Some(self.into()),
            // If this node still has a value but no longer has children, convert it into a leaf node
            (None, true) => Some(LeafNode::new(self.path, self.value).into()),
            // If this node doesn't have a value, replace it with its child node
            (Some(x), false) => Some(
                state
                    .get_node(x.clone())?
                    .expect("inconsistent internal tree structure"),
            ),
            // Return this node
            (None, false) => Some(self.into()),
        };

        Ok((new_node, value))
    }

    /// Computes the node's hash
    pub fn compute_hash(&self) -> NodeHash {
        NodeHash::from_encoded_raw(self.encode_raw())
    }

    /// Encodes the node
    pub fn encode_raw(&self) -> Vec<u8> {
        let hash_choice = |node_hash: &NodeHash| -> (Vec<u8>, usize) {
            if node_hash.is_valid() {
                match node_hash {
                    NodeHash::Hashed(x) => (x.as_bytes().to_vec(), 32),
                    NodeHash::Inline(x) => (x.clone(), x.len()),
                }
            } else {
                (Vec::new(), 0)
            }
        };
        let children = self.choices.iter().map(hash_choice).collect::<Vec<_>>();
        let encoded_value = (!self.value.is_empty()).then_some(&self.value[..]);

        let mut children_len: usize = children
            .iter()
            .map(|x| match x {
                (_, 0) => 1,
                (x, 32) => NodeEncoder::bytes_len(32, x[0]),
                (_, y) => *y,
            })
            .sum();
        if let Some(value) = encoded_value {
            children_len +=
                NodeEncoder::bytes_len(value.len(), value.first().copied().unwrap_or_default());
        } else {
            children_len += 1;
        }

        let mut encoder = NodeEncoder::new();
        encoder.write_list_header(children_len);
        children.iter().for_each(|(x, len)| match len {
            0 => encoder.write_bytes(&[]),
            32 => encoder.write_bytes(x),
            _ => encoder.write_raw(x),
        });
        match encoded_value {
            Some(value) => encoder.write_bytes(value),
            None => encoder.write_bytes(&[]),
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
        // Check the corresponding choice and delegate accordingly if present.
        if let Some(choice) = path.next().map(usize::from) {
            // Continue to child
            let child_hash = &self.choices[choice];
            if child_hash.is_valid() {
                let child_node = state
                    .get_node(child_hash.clone())?
                    .expect("inconsistent internal tree structure");
                child_node.get_path(state, path, node_path)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use ethereum_types::H256;

    use super::*;

    use crate::{pmt_node, Trie};

    #[test]
    fn new() {
        let node = BranchNode::new({
            let mut choices = BranchNode::EMPTY_CHOICES;

            choices[2] = NodeHash::Hashed(H256([2; 32]));
            choices[5] = NodeHash::Hashed(H256([5; 32]));

            Box::new(choices)
        });

        assert_eq!(
            *node.choices,
            [
                Default::default(),
                Default::default(),
                NodeHash::Hashed(H256([2; 32])),
                Default::default(),
                Default::default(),
                NodeHash::Hashed(H256([5; 32])),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
                Default::default(),
            ],
        );
    }

    #[test]
    fn get_some() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x10] => vec![0x34, 0x56, 0x78, 0x9A] },
            }
        };

        assert_eq!(
            node.get(&trie.state, NibbleSlice::new(&[0x00])).unwrap(),
            Some(vec![0x12, 0x34, 0x56, 0x78]),
        );
        assert_eq!(
            node.get(&trie.state, NibbleSlice::new(&[0x10])).unwrap(),
            Some(vec![0x34, 0x56, 0x78, 0x9A]),
        );
    }

    #[test]
    fn get_none() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x10] => vec![0x34, 0x56, 0x78, 0x9A] },
            }
        };

        assert_eq!(
            node.get(&trie.state, NibbleSlice::new(&[0x20])).unwrap(),
            None,
        );
    }

    #[test]
    fn insert_self() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x10] => vec![0x34, 0x56, 0x78, 0x9A] },
            }
        };
        let path = NibbleSlice::new(&[0x2]);
        let value = vec![0x3];

        let node = node
            .insert(&mut trie.state, path.clone(), value.clone())
            .unwrap();

        assert!(matches!(node, Node::Branch(_)));
        assert_eq!(node.get(&trie.state, path).unwrap(), Some(value));
    }

    #[test]
    fn insert_choice() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x10] => vec![0x34, 0x56, 0x78, 0x9A] },
            }
        };

        let path = NibbleSlice::new(&[0x20]);
        let value = vec![0x21];

        let node = node
            .insert(&mut trie.state, path.clone(), value.clone())
            .unwrap();

        assert!(matches!(node, Node::Branch(_)));
        assert_eq!(node.get(&trie.state, path).unwrap(), Some(value));
    }

    #[test]
    fn insert_passthrough() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x10] => vec![0x34, 0x56, 0x78, 0x9A] },
            }
        };

        // The extension node is ignored since it's irrelevant in this test.
        let mut path = NibbleSlice::new(&[0x00]);
        path.offset_add(2);
        let value = vec![0x1];

        let new_node = node
            .clone()
            .insert(&mut trie.state, path.clone(), value.clone())
            .unwrap();

        let new_node = match new_node {
            Node::Branch(x) => x,
            _ => panic!("expected a branch node"),
        };

        assert_eq!(new_node.choices, node.choices);
        assert_eq!(new_node.path, path.data());
        assert_eq!(new_node.value, value);
    }

    #[test]
    fn remove_choice_into_inner() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x00] },
                1 => leaf { vec![0x10] => vec![0x10] },
            }
        };

        let (node, value) = node
            .remove(&mut trie.state, NibbleSlice::new(&[0x00]))
            .unwrap();

        assert!(matches!(node, Some(Node::Leaf(_))));
        assert_eq!(value, Some(vec![0x00]));
    }

    #[test]
    fn remove_choice() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x00] },
                1 => leaf { vec![0x10] => vec![0x10] },
                2 => leaf { vec![0x10] => vec![0x10] },
            }
        };

        let (node, value) = node
            .remove(&mut trie.state, NibbleSlice::new(&[0x00]))
            .unwrap();

        assert!(matches!(node, Some(Node::Branch(_))));
        assert_eq!(value, Some(vec![0x00]));
    }

    #[test]
    fn remove_choice_into_value() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x00] },
            } with_leaf { vec![0x01] => vec![0xFF] }
        };

        let (node, value) = node
            .remove(&mut trie.state, NibbleSlice::new(&[0x00]))
            .unwrap();

        assert!(matches!(node, Some(Node::Leaf(_))));
        assert_eq!(value, Some(vec![0x00]));
    }

    #[test]
    fn remove_value_into_inner() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x00] },
            } with_leaf { vec![0x1] => vec![0xFF] }
        };

        let (node, value) = node.remove(&mut trie.state, NibbleSlice::new(&[])).unwrap();

        assert!(matches!(node, Some(Node::Leaf(_))));
        assert_eq!(value, Some(vec![0xFF]));
    }

    #[test]
    fn remove_value() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x00] },
                1 => leaf { vec![0x10] => vec![0x10] },
            } with_leaf { vec![0x1] => vec![0xFF] }
        };

        let (node, value) = node.remove(&mut trie.state, NibbleSlice::new(&[])).unwrap();

        assert!(matches!(node, Some(Node::Branch(_))));
        assert_eq!(value, Some(vec![0xFF]));
    }

    #[test]
    fn compute_hash_two_choices() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            branch {
                2 => leaf { vec![0x20] => vec![0x20] },
                4 => leaf { vec![0x40] => vec![0x40] },
            }
        };

        assert_eq!(
            node.compute_hash().as_ref(),
            &[
                0xD5, 0x80, 0x80, 0xC2, 0x30, 0x20, 0x80, 0xC2, 0x30, 0x40, 0x80, 0x80, 0x80, 0x80,
                0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
            ],
        );
    }

    #[test]
    fn compute_hash_all_choices() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            branch {
                0x0 => leaf { vec![0x00] => vec![0x00] },
                0x1 => leaf { vec![0x10] => vec![0x10] },
                0x2 => leaf { vec![0x20] => vec![0x20] },
                0x3 => leaf { vec![0x30] => vec![0x30] },
                0x4 => leaf { vec![0x40] => vec![0x40] },
                0x5 => leaf { vec![0x50] => vec![0x50] },
                0x6 => leaf { vec![0x60] => vec![0x60] },
                0x7 => leaf { vec![0x70] => vec![0x70] },
                0x8 => leaf { vec![0x80] => vec![0x80] },
                0x9 => leaf { vec![0x90] => vec![0x90] },
                0xA => leaf { vec![0xA0] => vec![0xA0] },
                0xB => leaf { vec![0xB0] => vec![0xB0] },
                0xC => leaf { vec![0xC0] => vec![0xC0] },
                0xD => leaf { vec![0xD0] => vec![0xD0] },
                0xE => leaf { vec![0xE0] => vec![0xE0] },
                0xF => leaf { vec![0xF0] => vec![0xF0] },
            }
        };

        assert_eq!(
            node.compute_hash().as_ref(),
            &[
                0x0A, 0x3C, 0x06, 0x2D, 0x4A, 0xE3, 0x61, 0xEC, 0xC4, 0x82, 0x07, 0xB3, 0x2A, 0xDB,
                0x6A, 0x3A, 0x3F, 0x3E, 0x98, 0x33, 0xC8, 0x9C, 0x9A, 0x71, 0x66, 0x3F, 0x4E, 0xB5,
                0x61, 0x72, 0xD4, 0x9D,
            ],
        );
    }

    #[test]
    fn compute_hash_one_choice_with_value() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            branch {
                2 => leaf { vec![0x20] => vec![0x20] },
                4 => leaf { vec![0x40] => vec![0x40] },
            } with_leaf { vec![0x1] => vec![0x1] }
        };

        assert_eq!(
            node.compute_hash().as_ref(),
            &[
                0xD5, 0x80, 0x80, 0xC2, 0x30, 0x20, 0x80, 0xC2, 0x30, 0x40, 0x80, 0x80, 0x80, 0x80,
                0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01,
            ],
        );
    }

    #[test]
    fn compute_hash_all_choices_with_value() {
        let mut trie = Trie::new_temp();
        let node = pmt_node! { @(trie)
            branch {
                0x0 => leaf { vec![0x00] => vec![0x00] },
                0x1 => leaf { vec![0x10] => vec![0x10] },
                0x2 => leaf { vec![0x20] => vec![0x20] },
                0x3 => leaf { vec![0x30] => vec![0x30] },
                0x4 => leaf { vec![0x40] => vec![0x40] },
                0x5 => leaf { vec![0x50] => vec![0x50] },
                0x6 => leaf { vec![0x60] => vec![0x60] },
                0x7 => leaf { vec![0x70] => vec![0x70] },
                0x8 => leaf { vec![0x80] => vec![0x80] },
                0x9 => leaf { vec![0x90] => vec![0x90] },
                0xA => leaf { vec![0xA0] => vec![0xA0] },
                0xB => leaf { vec![0xB0] => vec![0xB0] },
                0xC => leaf { vec![0xC0] => vec![0xC0] },
                0xD => leaf { vec![0xD0] => vec![0xD0] },
                0xE => leaf { vec![0xE0] => vec![0xE0] },
                0xF => leaf { vec![0xF0] => vec![0xF0] },
            } with_leaf { vec![0x1] => vec![0x1] }
        };

        assert_eq!(
            node.compute_hash().as_ref(),
            &[
                0x2A, 0x85, 0x67, 0xC5, 0x63, 0x4A, 0x87, 0xBA, 0x19, 0x6F, 0x2C, 0x65, 0x15, 0x16,
                0x66, 0x37, 0xE0, 0x9A, 0x34, 0xE6, 0xC9, 0xB0, 0x4D, 0xA5, 0x6F, 0xC4, 0x70, 0x4E,
                0x38, 0x61, 0x7D, 0x8E
            ],
        );
    }
}
