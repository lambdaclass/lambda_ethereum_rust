use crate::{
    error::StoreError,
    trie::{
        db::{PathRLP, TrieDB, ValueRLP},
        nibble::NibbleSlice,
        node::BranchNode,
    },
};

use super::{InsertAction, Node, NodeHash};

#[derive(Debug, Clone)]
pub struct LeafNode {
    pub hash: NodeHash,
    pub path: PathRLP,
}

impl LeafNode {
    pub fn new(path: PathRLP) -> Self {
        Self {
            hash: Default::default(),
            path,
        }
    }

    pub fn update_path(&mut self, new_path: PathRLP) {
        self.path = new_path
    }

    pub fn get(&self, db: &TrieDB, path: NibbleSlice) -> Result<Option<ValueRLP>, StoreError> {
        if path.cmp_rest(&self.path) {
            db.get_value(self.path.clone())
        } else {
            Ok(None)
        }
    }

    pub fn insert(&mut self, db: &TrieDB, path: NibbleSlice) -> (Node, InsertAction) {
        // Mark hash as dirty
        self.hash = Default::default();
        if path.cmp_rest(&self.path) {
            return (
                Node::Leaf(self.clone()),
                InsertAction::Replace(self.path.clone()),
            );
        } else {
            let offset = path.clone().count_prefix_slice(&{
                let mut value_path = NibbleSlice::new(&self.path);
                value_path.offset_add(path.offset());
                value_path
            });

            let mut path_branch = path.clone();
            path_branch.offset_add(offset);

            let absolute_offset = path_branch.offset();
            // let (branch_node, mut insert_action) = if absolute_offset == 2 * path.as_ref().len() {
            //     let child_ref =
            // } else {
            //     // BranchNode::new( {
            //     //     let mut choices = Default::default();
            //     //     choices[NibbleSlice::new(&self.path).nth(absolute_offset).unwrap() as usize] =

            //     // })
            // };
        }
        todo!()
    }
}
