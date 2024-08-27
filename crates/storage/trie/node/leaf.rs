use crate::{
    error::StoreError,
    trie::{
        db::{PathRLP, TrieDB, ValueRLP},
        nibble::NibbleSlice,
        node::BranchNode,
    },
};

use super::{ExtensionNode, InsertAction, Node, NodeHash};

#[derive(Debug, Clone, Default)]
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

    pub fn insert(
        &mut self,
        db: &mut TrieDB,
        path: NibbleSlice,
    ) -> Result<(Node, InsertAction), StoreError> {
        // Mark hash as dirty
        self.hash = Default::default();
        if path.cmp_rest(&self.path) {
            return Ok((
                Node::Leaf(self.clone()),
                InsertAction::Replace(self.path.clone()),
            ));
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
                (
                    BranchNode::new({
                        let mut choices = [Default::default(); 16];
                        // TODO: Dedicated method.
                        choices[NibbleSlice::new(self.path.as_ref())
                            .nth(absolute_offset)
                            .unwrap() as usize] = db.insert_node(self.clone().into())?;
                        choices
                    }),
                    InsertAction::InsertSelf,
                )
            } else if absolute_offset == 2 * self.path.len() {
                let child_ref = db.insert_node(LeafNode::default().into())?;
                let mut branch_node = BranchNode::new({
                    let mut choices = [Default::default(); 16];
                    choices[path_branch.next().unwrap() as usize] = child_ref;
                    choices
                });
                branch_node.update_path(self.path.clone());

                (branch_node, InsertAction::Insert(child_ref))
            } else {
                let child_ref = db.insert_node(LeafNode::default().into())?;
                (
                    BranchNode::new({
                        let mut choices = [Default::default(); 16];
                        choices[NibbleSlice::new(self.path.as_ref())
                            .nth(absolute_offset)
                            .unwrap() as usize] = db.insert_node(self.clone().into())?;
                        choices[path_branch.next().unwrap() as usize] = child_ref;
                        choices
                    }),
                    InsertAction::Insert(child_ref),
                )
            };

            let final_node = if offset != 0 {
                let branch_ref = db.insert_node(Node::Branch(branch_node.into()))?;
                insert_action = insert_action.quantize_self(branch_ref);

                ExtensionNode::new(path.split_to_vec(offset), branch_ref).into()
            } else {
                branch_node.into()
            };

            Ok((final_node, insert_action))
        }
    }
}
