use ethereum_types::H256;

use crate::error::StoreError;

use super::{
    db::{PathRLP, TrieDB, ValueRLP},
    nibble::NibbleVec,
};
pub type NodeHash = H256;

pub enum Node {
    Branch(BranchNode),
    Extension(ExtensionNode),
    Leaf(LeafNode),
}

pub struct BranchNode {
    pub hash: NodeHash,
    pub choices: [NodeHash; 16],
}

pub struct ExtensionNode {
    pub hash: NodeHash,
    pub prefix: NibbleVec,
    pub child: NodeHash,
}

#[derive(Debug, Clone)]
pub struct LeafNode {
    pub hash: NodeHash,
    pub path: PathRLP,
}

pub enum InsertAction {
    Replace(PathRLP),
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

    pub fn get(&self, db: &TrieDB, path: PathRLP) -> Result<Option<ValueRLP>, StoreError> {
        if path == self.path {
            db.get_value(path)
        } else {
            Ok(None)
        }
    }

    pub fn insert(&mut self, db: &TrieDB, path: PathRLP) -> (Node, InsertAction) {
        // Mark hash as dirty
        self.hash = Default::default();
        if path == self.path {
            return (
                Node::Leaf(self.clone()),
                InsertAction::Replace(self.path.clone()),
            );
        } else {
        }
        todo!()
    }
}
