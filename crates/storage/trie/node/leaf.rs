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
        mut self,
        db: &mut TrieDB,
        path: NibbleSlice,
    ) -> Result<(Node, InsertAction), StoreError> {
        // Possible flow paths:
        //   leaf { path => value } -> leaf { path => value }
        //   leaf { path => value } -> branch { 0 => leaf { path => value }, 1 => leaf { path => value } }
        //   leaf { path => value } -> extension { [0], branch { 0 => leaf { path => value }, 1 => leaf { path => value } } }
        //   leaf { path => value } -> extension { [0], branch { 0 => leaf { path => value } } with_value leaf { path => value } }
        //   leaf { path => value } -> extension { [0], branch { 0 => leaf { path => value } } with_value leaf { path => value } } // leafs swapped

        self.hash.mark_as_dirty();

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
