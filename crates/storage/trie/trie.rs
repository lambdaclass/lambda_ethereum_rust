use sha3::{Digest, Keccak256};

use super::{
    db::{PathRLP, TrieDB, ValueRLP},
    hashing::{NodeHashRef, Output},
    nibble::NibbleSlice,
    node::{InsertAction, LeafNode, Node},
    node_ref::NodeRef,
};
use crate::error::StoreError;

pub struct Trie {
    /// Root node ref.
    root_ref: NodeRef,
    /// Contains all the nodes and all the node's values
    pub(crate) db: TrieDB,
    hash: (bool, Output),
}

impl Trie {
    pub fn new(trie_dir: &str) -> Result<Self, StoreError> {
        Ok(Self {
            root_ref: NodeRef::default(),
            db: TrieDB::init(trie_dir)?,
            hash: (false, Default::default()),
        })
    }

    /// Retrieve a value from the tree given its path.
    /// TODO: Make inputs T: RLPEncode (we will ignore generics for now)
    pub fn get(&self, path: &PathRLP) -> Result<Option<ValueRLP>, StoreError> {
        if !self.root_ref.is_valid() {
            return Ok(None);
        }
        let root_node = self
            .db
            .get_node(self.root_ref)?
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
        if let Some(root_node) = self.db.remove_node(self.root_ref)? {
            // If the tree is not empty, call the root node's insertion logic
            let (root_node, insert_action) =
                root_node.insert(&mut self.db, NibbleSlice::new(&path))?;
            self.root_ref = self.db.insert_node(root_node)?;

            match insert_action.quantize_self(self.root_ref) {
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
            self.root_ref = self.db.insert_node(LeafNode::new(path).into())?;
            Ok(None)
        }
    }

    /// Remove a value from the tree.
    pub fn remove(&mut self, path: PathRLP) -> Result<Option<ValueRLP>, StoreError> {
        if !self.root_ref.is_valid() {
            return Ok(None);
        }

        let root_node = self
            .db
            .remove_node(self.root_ref)?
            .expect("inconsistent internal tree structure");
        let (root_node, old_value) = root_node.remove(&mut self.db, NibbleSlice::new(&path))?;
        self.root_ref = match root_node {
            Some(root_node) => self.db.insert_node(root_node)?,
            None => Default::default(),
        };

        Ok(old_value)
    }

    /// Return the root hash of the tree (or recompute if needed).
    pub fn compute_hash(&mut self) -> Result<Output, StoreError> {
        if !self.hash.0 {
            if self.root_ref.is_valid() {
                let root_node = self
                    .db
                    .get_node(self.root_ref)?
                    .expect("inconsistent internal tree structure");

                match root_node.compute_hash(&self.db, 0)? {
                    NodeHashRef::Inline(x) => {
                        Keccak256::new()
                            .chain_update(&*x)
                            .finalize_into(&mut self.hash.1);
                    }
                    NodeHashRef::Hashed(x) => self.hash.1.copy_from_slice(&x.clone()),
                };
            } else {
                Keccak256::new()
                    .chain_update([0x80])
                    .finalize_into(&mut self.hash.1);
            }
            self.hash.0 = true;
        }
        Ok(self.hash.1)
    }
}

#[cfg(test)]
mod test {
    use crate::trie::test_utils::{remove_trie, start_trie};

    use super::*;

    const TRIE_TEST_DIR: &str = "trie-test-db";

    fn run_test(test: &dyn Fn(Trie)) {
        let trie = start_trie(TRIE_TEST_DIR);
        test(trie);
        remove_trie(TRIE_TEST_DIR)
    }

    #[test]
    fn run_trie_test_suite() {
        run_test(&compute_hash);
        run_test(&compute_hash_long);
        run_test(&get_insert_words);
        run_test(&get_insert_zero);
        run_test(&get_insert_a);
        run_test(&get_insert_b);
        run_test(&get_insert_c);
        run_test(&get_insert_d);
        run_test(&get_insert_e);
        run_test(&get_insert_f);
        run_test(&compute_hash_a);
    }

    fn compute_hash(mut trie: Trie) {
        trie.insert(b"first".to_vec(), b"value".to_vec()).unwrap();
        trie.insert(b"second".to_vec(), b"value".to_vec()).unwrap();

        assert_eq!(
            trie.compute_hash().unwrap().as_ref(),
            hex::decode("f7537e7f4b313c426440b7fface6bff76f51b3eb0d127356efbe6f2b3c891501")
                .unwrap(),
        );
    }

    fn compute_hash_long(mut trie: Trie) {
        trie.insert(b"first".to_vec(), b"value".to_vec()).unwrap();
        trie.insert(b"second".to_vec(), b"value".to_vec()).unwrap();
        trie.insert(b"third".to_vec(), b"value".to_vec()).unwrap();
        trie.insert(b"fourth".to_vec(), b"value".to_vec()).unwrap();

        assert_eq!(
            trie.compute_hash().unwrap().as_slice(),
            hex::decode("e2ff76eca34a96b68e6871c74f2a5d9db58e59f82073276866fdd25e560cedea")
                .unwrap(),
        );
    }

    fn get_insert_words(mut trie: Trie) {
        trie.insert(b"first".to_vec(), b"value".to_vec()).unwrap();
        trie.insert(b"second".to_vec(), b"value".to_vec()).unwrap();

        let first = trie.get(&&b"first"[..].to_vec()).unwrap();
        assert!(first.is_some());
        let second = trie.get(&&b"second"[..].to_vec()).unwrap();
        assert!(second.is_some());
    }

    fn get_insert_zero(mut trie: Trie) {
        trie.insert(vec![0x0], b"value".to_vec()).unwrap();
        let first = trie.get(&&[0x0][..].to_vec()).unwrap();
        assert!(first.is_some());
    }

    // shrinks to paths = [[16], [16, 0]], values = [[0], [0]]
    fn get_insert_a(mut trie: Trie) {
        trie.insert(vec![16], vec![0]).unwrap();
        trie.insert(vec![16, 0], vec![0]).unwrap();

        let item = trie.get(&vec![16]).unwrap();
        assert!(item.is_some());
        assert_eq!(item.unwrap(), vec![0]);

        let item = trie.get(&vec![16, 0]).unwrap();
        assert!(item.is_some());
        assert_eq!(item.unwrap(), vec![0]);
    }

    // # shrinks to paths = {[1, 0], [0, 0]}
    fn get_insert_b(mut trie: Trie) {
        trie.insert(vec![0, 0], vec![0, 0]).unwrap();
        trie.insert(vec![1, 0], vec![1, 0]).unwrap();

        let item = trie.get(&vec![1, 0]).unwrap();
        assert!(item.is_some());
        assert_eq!(item.unwrap(), vec![1, 0]);

        let item = trie.get(&vec![0, 0]).unwrap();
        assert!(item.is_some());
        assert_eq!(item.unwrap(), vec![0, 0]);
    }

    fn get_insert_c(mut trie: Trie) {
        trie.insert(vec![26, 192, 44, 251], vec![26, 192, 44, 251])
            .unwrap();
        trie.insert(
            vec![195, 132, 220, 124, 112, 201, 70, 128, 235],
            vec![195, 132, 220, 124, 112, 201, 70, 128, 235],
        )
        .unwrap();
        trie.insert(vec![126, 138, 25, 245, 146], vec![126, 138, 25, 245, 146])
            .unwrap(); // inserted here
        trie.insert(
            vec![129, 176, 66, 2, 150, 151, 180, 60, 124],
            vec![129, 176, 66, 2, 150, 151, 180, 60, 124],
        )
        .unwrap();
        trie.insert(vec![138, 101, 157], vec![138, 101, 157])
            .unwrap();

        let item = trie.get(&vec![26, 192, 44, 251]).unwrap();
        assert!(item.is_some());
        assert_eq!(item.unwrap(), vec![26, 192, 44, 251]);

        let item = trie
            .get(&vec![195, 132, 220, 124, 112, 201, 70, 128, 235])
            .unwrap();
        assert!(item.is_some());
        assert_eq!(
            item.unwrap(),
            vec![195, 132, 220, 124, 112, 201, 70, 128, 235]
        );

        let item = trie.get(&vec![126, 138, 25, 245, 146]).unwrap();
        assert!(item.is_some()); // dis fails
        assert_eq!(item.unwrap(), vec![126, 138, 25, 245, 146]);

        let item = trie
            .get(&vec![129, 176, 66, 2, 150, 151, 180, 60, 124])
            .unwrap();
        assert!(item.is_some());
        assert_eq!(item.unwrap(), vec![129, 176, 66, 2, 150, 151, 180, 60, 124]);

        let item = trie.get(&vec![138, 101, 157]).unwrap();
        assert!(item.is_some());
        assert_eq!(item.unwrap(), vec![138, 101, 157]);
    }

    fn get_insert_d(mut trie: Trie) {
        let vecs = vec![
            vec![52, 53, 143, 52, 206, 112],
            vec![14, 183, 34, 39, 113],
            vec![55, 5],
            vec![134, 123, 19],
            vec![0, 59, 240, 89, 83, 167],
            vec![22, 41],
            vec![13, 166, 159, 101, 90, 234, 91],
            vec![31, 180, 161, 122, 115, 51, 37, 61, 101],
            vec![208, 192, 4, 12, 163, 254, 129, 206, 109],
        ];
        for x in &vecs {
            trie.insert(x.clone(), x.clone()).unwrap();
        }
        for x in &vecs {
            let item = trie.get(x).unwrap();
            assert!(item.is_some());
            assert_eq!(item.unwrap(), *x);
        }
    }

    fn get_insert_e(mut trie: Trie) {
        trie.insert(vec![0x00], vec![0x00]).unwrap();
        trie.insert(vec![0xC8], vec![0xC8]).unwrap();
        trie.insert(vec![0xC8, 0x00], vec![0xC8, 0x00]).unwrap();

        assert_eq!(trie.get(&vec![0x00]).unwrap(), Some(vec![0x00]));
        assert_eq!(trie.get(&vec![0xC8]).unwrap(), Some(vec![0xC8]));
        assert_eq!(trie.get(&vec![0xC8, 0x00]).unwrap(), Some(vec![0xC8, 0x00]));
    }

    fn get_insert_f(mut trie: Trie) {
        trie.insert(vec![0x00], vec![0x00]).unwrap();
        trie.insert(vec![0x01], vec![0x01]).unwrap();
        trie.insert(vec![0x10], vec![0x10]).unwrap();
        trie.insert(vec![0x19], vec![0x19]).unwrap();
        trie.insert(vec![0x19, 0x00], vec![0x19, 0x00]).unwrap();
        trie.insert(vec![0x1A], vec![0x1A]).unwrap();

        assert_eq!(trie.get(&vec![0x00]).unwrap(), Some(vec![0x00]));
        assert_eq!(trie.get(&vec![0x01]).unwrap(), Some(vec![0x01]));
        assert_eq!(trie.get(&vec![0x10]).unwrap(), Some(vec![0x10]));
        assert_eq!(trie.get(&vec![0x19]).unwrap(), Some(vec![0x19]));
        assert_eq!(trie.get(&vec![0x19, 0x00]).unwrap(), Some(vec![0x19, 0x00]));
        assert_eq!(trie.get(&vec![0x1A]).unwrap(), Some(vec![0x1A]));
    }

    fn compute_hash_a(mut trie: Trie) {
        trie.insert("do".as_bytes().to_vec(), "verb".as_bytes().to_vec())
            .unwrap();
        trie.insert("horse".as_bytes().to_vec(), "stallion".as_bytes().to_vec())
            .unwrap();
        trie.insert("doge".as_bytes().to_vec(), "coin".as_bytes().to_vec())
            .unwrap();
        trie.insert("dog".as_bytes().to_vec(), "puppy".as_bytes().to_vec())
            .unwrap();

        assert_eq!(
            trie.compute_hash().unwrap().as_slice(),
            hex::decode("5991bb8c6514148a29db676a14ac506cd2cd5775ace63c30a4fe457715e9ac84")
                .unwrap()
                .as_slice()
        );
    }
}
