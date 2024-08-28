use super::{
    db::{PathRLP, TrieDB, ValueRLP},
    nibble::NibbleSlice,
    node::{InsertAction, LeafNode, Node},
    node_ref::NodeRef,
};
use crate::error::StoreError;

//pub type WorldStateTrie = PatriciaMerkleTree<Vec<u8>, Vec<u8>, Keccak256>;

pub struct Trie {
    /// Root node hash.
    root: NodeRef,
    /// Contains all the nodes and all the node's values
    db: TrieDB,
    hash: (bool, u64),
}

impl Trie {
    pub fn new(trie_dir: &str) -> Result<Self, StoreError> {
        Ok(Self {
            root: NodeRef::default(),
            db: TrieDB::init(trie_dir)?,
            hash: (false, 0),
        })
    }

    /// Retrieve a value from the tree given its path.
    /// TODO: Make inputs T: RLPEncode (we will ignore generics for now)
    pub fn get(&self, path: &PathRLP) -> Result<Option<ValueRLP>, StoreError> {
        if !self.root.is_valid() {
            return Ok(None);
        }
        let root_node = self
            .db
            .get_node(self.root)?
            .expect("inconsistent internal tree structure");

        root_node.get(&self.db, NibbleSlice::new(&path))
    }

    /// Insert a value into the tree.
    /// TODO: Make inputs T: RLPEncode (we will ignore generics for now)
    pub fn insert(
        &mut self,
        path: PathRLP,
        value: ValueRLP,
    ) -> Result<Option<ValueRLP>, StoreError> {
        // Mark hash as dirty
        self.hash.0 = false;
        if let Some(root_node) = self.db.remove_node(self.root)? {
            // If the tree is not empty, call the root node's insertion logic
            let (root_node, insert_action) =
                root_node.insert(&mut self.db, NibbleSlice::new(&path))?;
            self.root = self.db.insert_node(root_node)?;

            match insert_action.quantize_self(self.root) {
                InsertAction::Insert(node_ref) => {
                    self.db.insert_value(path.clone(), value)?;
                    let node = match self
                        .db
                        .get_node(node_ref)? // [WARNING] get_mut
                        .expect("inconsistent internal tree structure")
                    {
                        Node::Leaf(mut leaf_node) => {
                            leaf_node.update_path(path);
                            leaf_node.into()
                        }
                        Node::Branch(mut branch_node) => {
                            branch_node.update_path(path);
                            branch_node.into()
                        }
                        _ => panic!("inconsistent internal tree structure"),
                    };
                    self.db.update_node(node_ref, node)?;

                    Ok(None)
                }
                InsertAction::Replace(path) => self.db.replace_value(path, value),
                _ => unreachable!(),
            }
        } else {
            // If the tree is empty, just add a leaf.
            self.db.insert_value(path.clone(), value)?;
            self.root = self.db.insert_node(LeafNode::new(path).into())?;
            Ok(None)
        }
    }
}
