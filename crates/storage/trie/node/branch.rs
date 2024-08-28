use crate::{
    error::StoreError,
    trie::{
        db::{PathRLP, TrieDB, ValueRLP},
        hashing::{DelimitedHash, NodeHash, NodeHashRef, NodeHasher, Output},
        nibble::{Nibble, NibbleSlice, NibbleVec},
        node_ref::NodeRef,
    },
};

use super::{ExtensionNode, InsertAction, LeafNode, Node};

#[derive(Debug, Clone)]
pub struct BranchNode {
    pub hash: NodeHash,
    pub choices: [NodeRef; 16],
    pub path: PathRLP,
}

impl BranchNode {
    pub fn new(choices: [NodeRef; 16]) -> Self {
        Self {
            choices,
            hash: Default::default(),
            path: Default::default(),
        }
    }

    pub fn update_path(&mut self, new_path: PathRLP) {
        self.path = new_path
    }

    pub fn get(&self, db: &TrieDB, mut path: NibbleSlice) -> Result<Option<ValueRLP>, StoreError> {
        // If path is at the end, return to its own value if present.
        // Otherwise, check the corresponding choice and delegate accordingly if present.
        if let Some(choice) = path.next().map(usize::from) {
            // Delegate to children if present
            let child_ref = self.choices[choice];
            if child_ref.is_valid() {
                let child_node = db
                    .get_node(child_ref)?
                    .expect("inconsistent internal tree structure");
                child_node.get(db, path)
            } else {
                Ok(None)
            }
        } else {
            // Return internal value if present.
            db.get_value(self.path.clone())
        }
    }

    pub fn insert(
        mut self,
        db: &mut TrieDB,
        mut path: NibbleSlice,
    ) -> Result<(Node, InsertAction), StoreError> {
        // If path is at the end, insert or replace its own value.
        // Otherwise, check the corresponding choice and insert or delegate accordingly.

        self.hash.mark_as_dirty();

        let insert_action = match path.next() {
            Some(choice) => match &mut self.choices[choice as usize] {
                choice_ref if !choice_ref.is_valid() => {
                    let child_ref = db.insert_node(LeafNode::default().into())?;

                    InsertAction::Insert(child_ref)
                }
                choice_ref => {
                    let child_node = db
                        .remove_node(*choice_ref)?
                        .expect("inconsistent internal tree structure");

                    let (child_node, insert_action) = child_node.insert(db, path)?;
                    *choice_ref = db.insert_node(child_node)?;

                    insert_action.quantize_self(*choice_ref)
                }
            },
            None => {
                if !self.path.is_empty() {
                    InsertAction::Replace(self.path.clone())
                } else {
                    InsertAction::InsertSelf
                }
            }
        };

        Ok((self.clone().into(), insert_action))
    }

    pub fn remove(
        mut self,
        db: &mut TrieDB,
        mut path: NibbleSlice,
    ) -> Result<(Option<Node>, Option<ValueRLP>), StoreError> {
        // Possible flow paths:
        //   branch { 2 choices } -> leaf/extension { ... }
        //   branch { 3+ choices } -> branch { ... }
        //   branch { 1 choice } with value -> leaf { ... }
        //   branch { 1 choice } with value -> leaf/extension { ... }
        //   branch { 2+ choices } with value -> branch { ... }

        let path_offset = path.offset();
        let value = match path.next() {
            Some(choice_index) => {
                if self.choices[choice_index as usize].is_valid() {
                    let child_node = db
                        .remove_node(self.choices[choice_index as usize])?
                        .expect("inconsistent internal tree structure");
                    let (child_node, old_value) = child_node.remove(db, path)?;
                    if let Some(child_node) = child_node {
                        self.choices[choice_index as usize] = db.insert_node(child_node)?;
                    }
                    old_value
                } else {
                    None
                }
            }
            None => {
                if !self.path.is_empty() {
                    let value = db.remove_value(self.path.clone())?;
                    self.path = Default::default();
                    value
                } else {
                    None
                }
            }
        };

        // An `Err(_)` means more than one choice. `Ok(Some(_))` and `Ok(None)` mean a single and no
        // choices respectively.
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

        let child_ref = match choice_count {
            Ok(Some((choice_index, child_ref))) => {
                let choice_index = Nibble::try_from(choice_index as u8).unwrap();
                let child_node = db
                    .get_node(*child_ref)?
                    .expect("inconsistent internal tree structure");

                match child_node {
                    Node::Branch(_) => {
                        *child_ref = db.insert_node(
                            ExtensionNode::new(
                                NibbleVec::from_single(choice_index, path_offset % 2 != 0),
                                *child_ref,
                            )
                            .into(),
                        )?;
                    }
                    Node::Extension(mut extension_node) => {
                        extension_node.prefix.prepend(choice_index);
                        // As this node was changed we need to update it on the DB
                        db.update_node(*child_ref, extension_node.into())?;
                    }
                    _ => {}
                }

                Some(child_ref)
            }
            _ => None,
        };

        if value.is_some() {
            self.hash.mark_as_dirty();
        }

        let new_node = match (child_ref, !self.path.is_empty()) {
            (Some(_), true) => Some(self.into()),
            (None, true) => Some(LeafNode::new(self.path).into()),
            (Some(x), false) => Some(
                db.remove_node(*x)?
                    .expect("inconsistent internal tree structure"),
            ),
            (None, false) => Some(self.into()),
        };

        Ok((new_node, value))
    }

    pub fn compute_hash(&self, db: &TrieDB, path_offset: usize) -> Result<NodeHashRef, StoreError> {
        if let Some(hash) = self.hash.extract_ref() {
            return Ok(hash);
        };
        let hash_choice = |node_ref: NodeRef| -> Result<DelimitedHash, StoreError> {
            if node_ref.is_valid() {
                let child_node = db
                    .get_node(node_ref)?
                    .expect("inconsistent internal tree structure");

                let mut target = Output::default();
                let target_len = match child_node.compute_hash(db, path_offset + 1)? {
                    NodeHashRef::Inline(x) => {
                        target[..x.len()].copy_from_slice(&x);
                        x.len()
                    }
                    NodeHashRef::Hashed(x) => {
                        target.copy_from_slice(&x);
                        x.len()
                    }
                };

                Ok(DelimitedHash(target, target_len))
            } else {
                Ok(DelimitedHash(Output::default(), 0))
            }
        };
        let children = self
            .choices
            .iter()
            .map(|node_ref| hash_choice(*node_ref))
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .unwrap();

        let encoded_value = db.get_value(self.path.clone())?;

        Ok(compute_branch_hash::<DelimitedHash>(
            &self.hash,
            &children,
            encoded_value.as_deref(),
        ))
    }
}

pub fn compute_branch_hash<'a, T>(
    hash: &'a NodeHash,
    choices: &[T; 16],
    value: Option<&[u8]>,
) -> NodeHashRef<'a>
where
    T: AsRef<[u8]>,
{
    let mut children_len: usize = choices
        .iter()
        .map(|x| match x.as_ref().len() {
            0 => 1,
            32 => NodeHasher::bytes_len(32, x.as_ref()[0]),
            x => x,
        })
        .sum();

    if let Some(value) = value {
        children_len +=
            NodeHasher::bytes_len(value.len(), value.first().copied().unwrap_or_default());
    } else {
        children_len += 1;
    }

    let mut hasher = NodeHasher::new(hash);
    hasher.write_list_header(children_len);
    choices.iter().for_each(|x| match x.as_ref().len() {
        0 => hasher.write_bytes(&[]),
        32 => hasher.write_bytes(x.as_ref()),
        _ => hasher.write_raw(x.as_ref()),
    });
    match value {
        Some(value) => hasher.write_bytes(value),
        None => hasher.write_bytes(&[]),
    }
    hasher.finalize()
}
