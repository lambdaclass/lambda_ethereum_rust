use crate::error::StoreError;
use crate::trie::db::{TrieDB, ValueRLP};
use crate::trie::nibble::NibbleSlice;
use crate::trie::{nibble::NibbleVec, node_ref::NodeRef};

use crate::trie::hashing::{NodeHash, NodeHashRef, NodeHasher, PathKind};

use super::{BranchNode, InsertAction, LeafNode, Node};

#[derive(Debug)]
pub struct ExtensionNode {
    pub hash: NodeHash,
    pub prefix: NibbleVec,
    pub child: NodeRef,
}

impl ExtensionNode {
    pub(crate) fn new(prefix: NibbleVec, child: NodeRef) -> Self {
        Self {
            prefix,
            child,
            hash: Default::default(),
        }
    }
    pub fn get(&self, db: &TrieDB, mut path: NibbleSlice) -> Result<Option<ValueRLP>, StoreError> {
        // If the path is prefixed by this node's prefix, delegate to its child.
        // Otherwise, no value is present.
        if path.skip_prefix(&self.prefix) {
            let child_node = db
                .get_node(self.child)?
                .expect("inconsistent internal tree structure");

            child_node.get(db, path)
        } else {
            Ok(None)
        }
    }

    pub fn insert(
        mut self,
        db: &mut TrieDB,
        mut path: NibbleSlice,
    ) -> Result<(Node, InsertAction), StoreError> {
        // Possible flow paths (there are duplicates between different prefix lengths):
        //   extension { [0], child } -> branch { 0 => child } with_value !
        //   extension { [0], child } -> extension { [0], child }
        //   extension { [0, 1], child } -> branch { 0 => extension { [1], child } } with_value !
        //   extension { [0, 1], child } -> extension { [0], branch { 1 => child } with_value ! }
        //   extension { [0, 1], child } -> extension { [0, 1], child }
        //   extension { [0, 1, 2], child } -> branch { 0 => extension { [1, 2], child } } with_value !
        //   extension { [0, 1, 2], child } -> extension { [0], branch { 1 => extension { [2], child } } with_value ! }
        //   extension { [0, 1, 2], child } -> extension { [0, 1], branch { 2 => child } with_value ! }
        //   extension { [0, 1, 2], child } -> extension { [0, 1, 2], child }

        self.hash.mark_as_dirty();

        if path.skip_prefix(&self.prefix) {
            let child_node = db
                .remove_node(self.child)?
                .expect("inconsistent internal tree structure");

            let (child_node, insert_action) = child_node.insert(db, path)?;
            self.child = db.insert_node(child_node)?;

            let insert_action = insert_action.quantize_self(self.child);
            Ok((self.into(), insert_action))
        } else {
            let offset = path.clone().count_prefix_vec(&self.prefix);
            path.offset_add(offset);
            let (left_prefix, choice, right_prefix) = self.prefix.split_extract_at(offset);

            let left_prefix = (!left_prefix.is_empty()).then_some(left_prefix);
            let right_prefix = (!right_prefix.is_empty()).then_some(right_prefix);

            // Prefix right node (if any, child is self.child_ref).
            let right_prefix_node = if let Some(right_prefix) = right_prefix {
                db.insert_node(ExtensionNode::new(right_prefix, self.child).into())?
            } else {
                self.child
            };

            // Branch node (child is prefix right or self.child_ref).
            let mut insert_node_ref = None;
            let branch_node = BranchNode::new({
                let mut choices = [Default::default(); 16];
                choices[choice as usize] = right_prefix_node;
                if let Some(c) = path.next() {
                    choices[c as usize] =
                        db.insert_node(LeafNode::new(Default::default()).into())?;
                    insert_node_ref = Some(choices[c as usize]);
                }
                choices
            });

            // Prefix left node (if any, child is branch_node).
            match left_prefix {
                Some(left_prefix) => {
                    let branch_ref = db.insert_node(branch_node.into())?;

                    Ok((
                        ExtensionNode::new(left_prefix, branch_ref).into(),
                        InsertAction::Insert(insert_node_ref.unwrap_or(branch_ref)),
                    ))
                }
                None => match insert_node_ref {
                    Some(child_ref) => Ok((branch_node.into(), InsertAction::Insert(child_ref))),
                    None => Ok((branch_node.into(), InsertAction::InsertSelf)),
                },
            }
        }
    }

    pub fn remove(
        mut self,
        db: &mut TrieDB,
        mut path: NibbleSlice,
    ) -> Result<(Option<Node>, Option<ValueRLP>), StoreError> {
        // Possible flow paths:
        //   - extension { a, branch { ... } } -> extension { a, branch { ... }}
        //   - extension { a, branch { ... } } -> extension { a + b, branch { ... }}
        //   - extension { a, branch { ... } } -> leaf { ... }

        if path.skip_prefix(&self.prefix) {
            let child_node = db
                .remove_node(self.child)?
                .expect("inconsistent internal tree structure");

            let (child_node, old_value) = child_node.remove(db, path)?;
            if old_value.is_some() {
                self.hash.mark_as_dirty();
            }
            let node = match child_node {
                None => None,
                Some(node) => Some(match node {
                    Node::Branch(branch_node) => {
                        self.child = db.insert_node(branch_node.into())?;
                        self.into()
                    }
                    Node::Extension(extension_node) => {
                        self.prefix.extend(&extension_node.prefix);
                        self.into()
                    }
                    Node::Leaf(leaf_node) => leaf_node.into(),
                }),
            };

            Ok((node, old_value))
        } else {
            Ok((Some(self.into()), None))
        }
    }

    pub fn compute_hash(&self, db: &TrieDB, path_offset: usize) -> Result<NodeHashRef, StoreError> {
        if let Some(hash) = self.hash.extract_ref() {
            return Ok(hash);
        };
        let child_node = db
            .get_node(self.child)?
            .expect("inconsistent internal tree structure");

        let child_hash_ref = child_node.compute_hash(db, path_offset + self.prefix.len())?;

        Ok(compute_extension_hash(
            &self.hash,
            &self.prefix,
            child_hash_ref,
        ))
    }
}

pub fn compute_extension_hash<'a>(
    hash: &'a NodeHash,
    prefix: &NibbleVec,
    child_hash_ref: NodeHashRef,
) -> NodeHashRef<'a> {
    let prefix_len = NodeHasher::path_len(prefix.len());
    let child_len = match &child_hash_ref {
        NodeHashRef::Inline(x) => x.len(),
        NodeHashRef::Hashed(x) => NodeHasher::bytes_len(x.len(), x[0]),
    };

    let mut hasher = NodeHasher::new(hash);
    hasher.write_list_header(prefix_len + child_len);
    hasher.write_path_vec(prefix, PathKind::Extension);
    match child_hash_ref {
        NodeHashRef::Inline(x) => hasher.write_raw(&x),
        NodeHashRef::Hashed(x) => hasher.write_bytes(&x),
    }
    hasher.finalize()
}
