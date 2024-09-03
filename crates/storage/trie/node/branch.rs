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
                    *choice_ref = child_ref;
                    InsertAction::Insert(child_ref)
                }
                choice_ref => {
                    let child_node = db
                        // [Note]: Original impl would remove
                        .get_node(*choice_ref)?
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
                    } else {
                        self.choices[choice_index as usize] = NodeRef::default();
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        pmt_node,
        trie::{
            test_utils::{remove_trie, start_trie},
            Trie,
        },
    };

    #[test]
    fn new() {
        let node = BranchNode::new({
            let mut choices = [Default::default(); 16];

            choices[2] = NodeRef::new(2);
            choices[5] = NodeRef::new(5);

            choices
        });

        assert_eq!(
            node.choices,
            [
                Default::default(),
                Default::default(),
                NodeRef::new(2),
                Default::default(),
                Default::default(),
                NodeRef::new(5),
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

    const BRANCH_TEST_DIR: &str = "branch-test-db";

    fn run_test(test: &dyn Fn(Trie)) {
        let trie = start_trie(BRANCH_TEST_DIR);
        test(trie);
        remove_trie(BRANCH_TEST_DIR)
    }

    #[test]
    fn run_branch_test_suite() {
        run_test(&get_some);
        run_test(&get_none);
        run_test(&insert_self);
        run_test(&insert_choice);
        run_test(&insert_passthrough);
        run_test(&remove_choice);
        run_test(&remove_choice_into_inner);
        run_test(&remove_choice_into_value);
        run_test(&remove_value);
        run_test(&remove_value_into_inner);
        run_test(&compute_hash_all_choices);
        run_test(&compute_hash_all_choices_with_value);
        run_test(&compute_hash_one_choice_with_value);
        run_test(&compute_hash_two_choices);
    }

    fn get_some(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x10] => vec![0x34, 0x56, 0x78, 0x9A] },
            }
        };

        assert_eq!(
            node.get(&trie.db, NibbleSlice::new(&[0x00])).unwrap(),
            Some(vec![0x12, 0x34, 0x56, 0x78]),
        );
        assert_eq!(
            node.get(&trie.db, NibbleSlice::new(&[0x10])).unwrap(),
            Some(vec![0x34, 0x56, 0x78, 0x9A]),
        );
    }

    fn get_none(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x10] => vec![0x34, 0x56, 0x78, 0x9A] },
            }
        };

        assert_eq!(node.get(&trie.db, NibbleSlice::new(&[0x20])).unwrap(), None,);
    }

    fn insert_self(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x10] => vec![0x34, 0x56, 0x78, 0x9A] },
            }
        };

        let (node, insert_action) = node.insert(&mut trie.db, NibbleSlice::new(&[])).unwrap();
        let _ = match node {
            Node::Branch(x) => x,
            _ => panic!("expected a branch node"),
        };
        assert_eq!(insert_action, InsertAction::InsertSelf);
    }

    fn insert_choice(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x10] => vec![0x34, 0x56, 0x78, 0x9A] },
            }
        };

        let (node, insert_action) = node
            .insert(&mut trie.db, NibbleSlice::new(&[0x20]))
            .unwrap();
        let _ = match node {
            Node::Branch(x) => x,
            _ => panic!("expected a branch node"),
        };
        assert_eq!(insert_action, InsertAction::Insert(NodeRef::new(2)));
    }

    fn insert_passthrough(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x12, 0x34, 0x56, 0x78] },
                1 => leaf { vec![0x10] => vec![0x34, 0x56, 0x78, 0x9A] },
            }
        };

        // The extension node is ignored since it's irrelevant in this test.
        let (node, insert_action) = node
            .insert(&mut trie.db, {
                let mut nibble_slice = NibbleSlice::new(&[0x00]);
                nibble_slice.offset_add(2);
                nibble_slice
            })
            .unwrap();
        let _ = match node {
            Node::Branch(x) => x,
            _ => panic!("expected a branch node"),
        };
        assert_eq!(insert_action, InsertAction::InsertSelf);
    }

    fn remove_choice_into_inner(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x00] },
                1 => leaf { vec![0x10] => vec![0x10] },
            }
        };

        let (node, value) = node
            .remove(&mut trie.db, NibbleSlice::new(&[0x00]))
            .unwrap();

        assert!(matches!(node, Some(Node::Leaf(_))));
        assert_eq!(value, Some(vec![0x00]));
    }

    fn remove_choice(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x00] },
                1 => leaf { vec![0x10] => vec![0x10] },
                2 => leaf { vec![0x10] => vec![0x10] },
            }
        };

        let (node, value) = node
            .remove(&mut trie.db, NibbleSlice::new(&[0x00]))
            .unwrap();

        assert!(matches!(node, Some(Node::Branch(_))));
        assert_eq!(value, Some(vec![0x00]));
    }

    fn remove_choice_into_value(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x00] },
            } with_leaf { vec![0x01] => vec![0xFF] }
        };

        let (node, value) = node
            .remove(&mut trie.db, NibbleSlice::new(&[0x00]))
            .unwrap();

        assert!(matches!(node, Some(Node::Leaf(_))));
        assert_eq!(value, Some(vec![0x00]));
    }

    fn remove_value_into_inner(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x00] },
            } with_leaf { vec![0x1] => vec![0xFF] }
        };

        let (node, value) = node.remove(&mut trie.db, NibbleSlice::new(&[])).unwrap();

        assert!(matches!(node, Some(Node::Leaf(_))));
        assert_eq!(value, Some(vec![0xFF]));
    }

    fn remove_value(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            branch {
                0 => leaf { vec![0x00] => vec![0x00] },
                1 => leaf { vec![0x10] => vec![0x10] },
            } with_leaf { vec![0x1] => vec![0xFF] }
        };

        let (node, value) = node.remove(&mut trie.db, NibbleSlice::new(&[])).unwrap();

        assert!(matches!(node, Some(Node::Branch(_))));
        assert_eq!(value, Some(vec![0xFF]));
    }

    fn compute_hash_two_choices(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            branch {
                2 => leaf { vec![0x20] => vec![0x20] },
                4 => leaf { vec![0x40] => vec![0x40] },
            }
        };

        assert_eq!(
            node.compute_hash(&trie.db, 0).unwrap().as_ref(),
            &[
                0xD5, 0x80, 0x80, 0xC2, 0x30, 0x20, 0x80, 0xC2, 0x30, 0x40, 0x80, 0x80, 0x80, 0x80,
                0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
            ],
        );
    }

    fn compute_hash_all_choices(mut trie: Trie) {
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
            node.compute_hash(&trie.db, 0).unwrap().as_ref(),
            &[
                0x0A, 0x3C, 0x06, 0x2D, 0x4A, 0xE3, 0x61, 0xEC, 0xC4, 0x82, 0x07, 0xB3, 0x2A, 0xDB,
                0x6A, 0x3A, 0x3F, 0x3E, 0x98, 0x33, 0xC8, 0x9C, 0x9A, 0x71, 0x66, 0x3F, 0x4E, 0xB5,
                0x61, 0x72, 0xD4, 0x9D,
            ],
        );
    }

    fn compute_hash_one_choice_with_value(mut trie: Trie) {
        let node = pmt_node! { @(trie)
            branch {
                2 => leaf { vec![0x20] => vec![0x20] },
                4 => leaf { vec![0x40] => vec![0x40] },
            } with_leaf { vec![0x1] => vec![0x1] }
        };

        assert_eq!(
            node.compute_hash(&trie.db, 0).unwrap().as_ref(),
            &[
                0xD5, 0x80, 0x80, 0xC2, 0x30, 0x20, 0x80, 0xC2, 0x30, 0x40, 0x80, 0x80, 0x80, 0x80,
                0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01,
            ],
        );
    }

    fn compute_hash_all_choices_with_value(mut trie: Trie) {
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
            node.compute_hash(&trie.db, 0).unwrap().as_ref(),
            &[
                0x2A, 0x85, 0x67, 0xC5, 0x63, 0x4A, 0x87, 0xBA, 0x19, 0x6F, 0x2C, 0x65, 0x15, 0x16,
                0x66, 0x37, 0xE0, 0x9A, 0x34, 0xE6, 0xC9, 0xB0, 0x4D, 0xA5, 0x6F, 0xC4, 0x70, 0x4E,
                0x38, 0x61, 0x7D, 0x8E
            ],
        );
    }
}
