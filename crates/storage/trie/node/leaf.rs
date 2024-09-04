use crate::{
    error::StoreError,
    trie::{
        db::{PathRLP, TrieDB, ValueRLP},
        hashing::{NodeHash, NodeHashRef, NodeHasher, PathKind},
        nibble::NibbleSlice,
        node::BranchNode,
    },
};

use super::{ExtensionNode, InsertAction, Node};

#[derive(Debug, Clone, Default)]
pub struct LeafNode {
    pub hash: NodeHash,
    pub path: PathRLP,
    pub value: ValueRLP,
}

impl LeafNode {
    pub fn new(path: PathRLP) -> Self {
        Self {
            hash: Default::default(),
            path,
            value: Default::default(),
        }
    }
    // TODO: move to new
    pub fn new_v2(path: PathRLP, value: ValueRLP) -> Self {
        Self {
            hash: Default::default(),
            path,
            value,
        }
    }

    pub fn update(&mut self, new_path: PathRLP, new_value: ValueRLP) {
        self.path = new_path;
        self.value = new_value;
    }

    pub fn get(&self, db: &TrieDB, path: NibbleSlice) -> Result<Option<ValueRLP>, StoreError> {
        if path.cmp_rest(&self.path) {
            db.get_value(self.path.clone())
        } else {
            Ok(None)
        }
    }

    pub fn insert(
        mut self,
        db: &mut TrieDB,
        path: NibbleSlice,
        value: ValueRLP,
    ) -> Result<(Node, InsertAction), StoreError> {
        // Possible flow paths:
        //   leaf { path => value } -> leaf { path => value }
        //   leaf { path => value } -> branch { 0 => leaf { path => value }, 1 => leaf { path => value } }
        //   leaf { path => value } -> extension { [0], branch { 0 => leaf { path => value }, 1 => leaf { path => value } } }
        //   leaf { path => value } -> extension { [0], branch { 0 => leaf { path => value } } with_value leaf { path => value } }
        //   leaf { path => value } -> extension { [0], branch { 0 => leaf { path => value } } with_value leaf { path => value } } // leafs swapped
        self.hash.mark_as_dirty();
        if path.cmp_rest(&self.path) {
            self.value = value;
            Ok((self.clone().into(), InsertAction::NoOp))
        } else {
            let offset = path.clone().count_prefix_slice(&{
                let mut value_path = NibbleSlice::new(&self.path);
                value_path.offset_add(path.offset());
                value_path
            });

            let mut path_branch = path.clone();
            path_branch.offset_add(offset);

            let absolute_offset = path_branch.offset();
            let (branch_node, mut insert_action) = if absolute_offset == 2 * path.as_ref().len() {
                let mut choices = [Default::default(); 16];
                // TODO: Dedicated method.
                choices[NibbleSlice::new(self.path.as_ref())
                    .nth(absolute_offset)
                    .unwrap() as usize] = db.insert_node(self.clone().into())?;

                let branch_node = BranchNode::new_v2(choices, path.data(), value.clone());

                (branch_node, InsertAction::NoOp)
            } else if absolute_offset == 2 * self.path.len() {
                let new_leaf = LeafNode::new_v2(path.data(), value.clone());

                let child_ref = db.insert_node(new_leaf.into())?;
                let branch_node = BranchNode::new_v2(
                    {
                        let mut choices = [Default::default(); 16];
                        choices[path_branch.next().unwrap() as usize] = child_ref;
                        choices
                    },
                    self.path.clone(),
                    self.value,
                );

                (branch_node, InsertAction::NoOp)
            } else {
                let new_leaf = LeafNode::new_v2(path.data(), value.clone());

                let child_ref = db.insert_node(new_leaf.into())?;
                (
                    BranchNode::new({
                        let mut choices = [Default::default(); 16];
                        choices[NibbleSlice::new(self.path.as_ref())
                            .nth(absolute_offset)
                            .unwrap() as usize] = db.insert_node(self.clone().into())?;
                        choices[path_branch.next().unwrap() as usize] = child_ref;
                        choices
                    }),
                    InsertAction::NoOp,
                )
            };

            let final_node = if offset != 0 {
                let branch_ref = db.insert_node(Node::Branch(branch_node))?;
                insert_action = insert_action.quantize_self(branch_ref);

                ExtensionNode::new(path.split_to_vec(offset), branch_ref).into()
            } else {
                branch_node.into()
            };

            Ok(dbg!((final_node, insert_action)))
        }
    }

    pub fn remove(
        self,
        db: &mut TrieDB,
        path: NibbleSlice,
    ) -> Result<(Option<Node>, Option<ValueRLP>), StoreError> {
        Ok(if path.cmp_rest(&self.path) {
            let value = db.remove_value(self.path.clone())?;
            (None, value)
        } else {
            (Some(self.into()), None)
        })
    }
    pub fn compute_hash(&self, db: &TrieDB, path_offset: usize) -> Result<NodeHashRef, StoreError> {
        if let Some(hash) = self.hash.extract_ref() {
            return Ok(hash);
        }
        let encoded_value = db
            .get_value(self.path.clone())?
            .expect("inconsistent internal tree structure");
        let encoded_path = &self.path;

        let mut path_slice = NibbleSlice::new(encoded_path);
        path_slice.offset_add(path_offset);

        Ok(compute_leaf_hash(
            &self.hash,
            path_slice,
            encoded_value.as_ref(),
        ))
    }
}

pub fn compute_leaf_hash<'a>(
    hash: &'a NodeHash,
    path: NibbleSlice,
    value: &[u8],
) -> NodeHashRef<'a> {
    let path_len = NodeHasher::path_len(path.len());
    let value_len = NodeHasher::bytes_len(value.len(), value.first().copied().unwrap_or_default());

    let mut hasher = NodeHasher::new(hash);
    hasher.write_list_header(path_len + value_len);
    hasher.write_path_slice(&path, PathKind::Leaf);
    hasher.write_bytes(value);
    hasher.finalize()
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::trie::node_ref::NodeRef;
    use crate::trie::Trie;
    use crate::{
        pmt_node,
        trie::test_utils::{remove_trie, start_trie},
    };

    #[test]
    fn new() {
        let node = LeafNode::new(Default::default());
        assert_eq!(node.path, PathRLP::default());
    }

    const LEAF_TEST_DIR: &str = "leaf-test-db";

    fn run_test(test: &dyn Fn(Trie)) {
        let trie = start_trie(LEAF_TEST_DIR);
        test(trie);
        remove_trie(LEAF_TEST_DIR)
    }

    #[test]
    fn run_leaf_test_suite() {
        run_test(&get_some);
        run_test(&get_none);
        run_test(&insert_replace);
        run_test(&insert_branch);
        run_test(&insert_extension_branch);
        run_test(&insert_extension_branch_value_self);
        run_test(&insert_extension_branch_value_other);
        run_test(&remove_self);
        run_test(&remove_none);
        run_test(&compute_hash);
        run_test(&compute_hash_long);
    }

    fn get_some(trie: Trie) {
        let node = pmt_node! { @(trie)
            leaf { vec![0x12] => vec![0x12, 0x34, 0x56, 0x78] }
        };

        assert_eq!(
            node.get(&trie.db, NibbleSlice::new(&[0x12])).unwrap(),
            Some(vec![0x12, 0x34, 0x56, 0x78]),
        );
    }

    fn get_none(trie: Trie) {
        let node = pmt_node! { @(trie)
            leaf { vec![0x12] => vec![0x12, 0x34, 0x56, 0x78] }
        };

        assert!(node
            .get(&trie.db, NibbleSlice::new(&[0x34]))
            .unwrap()
            .is_none());
    }

    fn insert_replace(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            leaf { vec![0x12] => vec![0x12, 0x34, 0x56, 0x78] }
        };

        let (node, insert_action) = node
            .insert(&mut trie.db, NibbleSlice::new(&[0x12]), vec![])
            .unwrap();
        let node = match node {
            Node::Leaf(x) => x,
            _ => panic!("expected a leaf node"),
        };

        assert_eq!(node.path, vec![0x12]);
        assert!(node.hash.extract_ref().is_none());
        assert_eq!(insert_action, InsertAction::Replace(vec![0x12]));
    }

    fn insert_branch(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            leaf { vec![0x12] => vec![0x12, 0x34, 0x56, 0x78] }
        };

        let (node, insert_action) = node
            .insert(&mut trie.db, NibbleSlice::new(&[0x22]), vec![])
            .unwrap();
        let _ = match node {
            Node::Branch(x) => x,
            _ => panic!("expected a branch node"),
        };
        assert_eq!(insert_action, InsertAction::Insert(NodeRef::new(0)));
    }

    fn insert_extension_branch(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            leaf { vec![0x12] => vec![0x12, 0x34, 0x56, 0x78] }
        };

        let (node, insert_action) = node
            .insert(&mut trie.db, NibbleSlice::new(&[0x13]), vec![])
            .unwrap();
        let _ = match node {
            Node::Extension(x) => x,
            _ => panic!("expected an extension node"),
        };
        assert_eq!(insert_action, InsertAction::Insert(NodeRef::new(0)));
    }

    fn insert_extension_branch_value_self(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            leaf { vec![0x12] => vec![0x12, 0x34, 0x56, 0x78] }
        };

        let (node, insert_action) = node
            .insert(&mut trie.db, NibbleSlice::new(&[0x12, 0x34]), vec![])
            .unwrap();
        let _ = match node {
            Node::Extension(x) => x,
            _ => panic!("expected an extension node"),
        };
        assert_eq!(insert_action, InsertAction::Insert(NodeRef::new(0)));
    }

    fn insert_extension_branch_value_other(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            leaf { vec![0x12, 0x34] => vec![0x12, 0x34, 0x56, 0x78] }
        };

        let (node, insert_action) = node
            .insert(&mut trie.db, NibbleSlice::new(&[0x12]), vec![])
            .unwrap();
        let _ = match node {
            Node::Extension(x) => x,
            _ => panic!("expected an extension node"),
        };
        assert_eq!(insert_action, InsertAction::Insert(NodeRef::new(1)));
    }

    // An insertion that returns branch [value=(x)] -> leaf (y) is not possible because of the path
    // restrictions: nibbles come in pairs. If the first nibble is different, the node will be a
    // branch but it cannot have a value. If the second nibble is different, then it'll be an
    // extension followed by a branch with value and a child.
    //
    // Because of that, the two tests that would check those cases are neither necessary nor
    // possible.

    fn remove_self(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            leaf { vec![0x12, 0x34] => vec![0x12, 0x34, 0x56, 0x78] }
        };

        let (node, value) = node
            .remove(&mut trie.db, NibbleSlice::new(&[0x12, 0x34]))
            .unwrap();

        assert!(node.is_none());
        assert_eq!(value, Some(vec![0x12, 0x34, 0x56, 0x78]));
    }

    fn remove_none(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            leaf { vec![0x12, 0x34] => vec![0x12, 0x34, 0x56, 0x78] }
        };

        let (node, value) = node
            .remove(&mut trie.db, NibbleSlice::new(&[0x12]))
            .unwrap();

        assert!(node.is_some());
        assert_eq!(value, None);
    }

    fn compute_hash(trie: Trie) {
        let node = pmt_node! { @(trie)
            leaf { b"key".to_vec() => b"value".to_vec() }
        };

        let node_hash_ref = node.compute_hash(&trie.db, 0).unwrap();
        assert_eq!(
            node_hash_ref.as_ref(),
            &[0xCB, 0x84, 0x20, 0x6B, 0x65, 0x79, 0x85, 0x76, 0x61, 0x6C, 0x75, 0x65],
        );
    }

    fn compute_hash_long(trie: Trie) {
        let node = pmt_node! { @(trie)
            leaf { b"key".to_vec() => b"a comparatively long value".to_vec() }
        };

        let node_hash_ref = node.compute_hash(&trie.db, 0).unwrap();
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
