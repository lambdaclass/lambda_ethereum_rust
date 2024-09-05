mod db;
mod hashing;
mod nibble;
mod node;
mod node_ref;
mod rlp;
#[cfg(test)]
mod test_utils;

use sha3::{Digest, Keccak256};

use self::{
    db::TrieDB,
    hashing::{NodeHashRef, Output},
    nibble::NibbleSlice,
    node::LeafNode,
    node_ref::NodeRef,
};
use crate::error::StoreError;

pub type PathRLP = Vec<u8>;
pub type ValueRLP = Vec<u8>;

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

        root_node.get(&self.db, NibbleSlice::new(path))
    }

    /// Insert a value into the tree.
    /// TODO: Make inputs T: RLPEncode (we will ignore generics for now)
    pub fn insert(&mut self, path: PathRLP, value: ValueRLP) -> Result<(), StoreError> {
        println!("[INSERT]: {:?}: {:?}", path, value);
        // Mark hash as dirty
        self.hash.0 = false;
        // [Note]: Original impl would remove
        if let Some(root_node) = self.db.get_node(self.root_ref)? {
            // If the tree is not empty, call the root node's insertion logic
            let root_node =
                root_node.insert(&mut self.db, NibbleSlice::new(&path), value.clone())?;
            self.root_ref = self.db.insert_node(root_node)?;
        } else {
            // If the tree is empty, just add a leaf.
            self.root_ref = self.db.insert_node(LeafNode::new(path, value).into())?;
        }
        Ok(())
    }

    /// Remove a value from the tree.
    pub fn remove(&mut self, path: PathRLP) -> Result<Option<ValueRLP>, StoreError> {
        println!("[REMOVE]: {:?}", path);
        if !self.root_ref.is_valid() {
            return Ok(None);
        }

        let root_node = self
            .db
            .get_node(self.root_ref)?
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

    #[cfg(test)]
    /// Creates a new trie based on a temporary DB
    pub fn new_temp() -> Self {
        Self {
            root_ref: NodeRef::default(),
            db: TrieDB::init_temp(),
            hash: (false, Default::default()),
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use super::*;
    // Rename imports to avoid potential name clashes
    use cita_trie::{MemoryDB as CitaMemoryDB, PatriciaTrie as CitaTrie, Trie as CitaTrieTrait};
    use hasher::HasherKeccak;
    use proptest::{
        collection::{btree_set, vec},
        prelude::*,
        proptest,
    };

    #[test]
    fn compute_hash() {
        let mut trie = Trie::new_temp();
        trie.insert(b"first".to_vec(), b"value".to_vec()).unwrap();
        trie.insert(b"second".to_vec(), b"value".to_vec()).unwrap();

        assert_eq!(
            trie.compute_hash().unwrap().as_ref(),
            hex::decode("f7537e7f4b313c426440b7fface6bff76f51b3eb0d127356efbe6f2b3c891501")
                .unwrap(),
        );
    }

    #[test]
    fn compute_hash_long() {
        let mut trie = Trie::new_temp();
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

    #[test]
    fn get_insert_words() {
        let mut trie = Trie::new_temp();
        trie.insert(b"first".to_vec(), b"value".to_vec()).unwrap();
        trie.insert(b"second".to_vec(), b"value".to_vec()).unwrap();

        let first = trie.get(&b"first"[..].to_vec()).unwrap();
        assert!(first.is_some());
        let second = trie.get(&b"second"[..].to_vec()).unwrap();
        assert!(second.is_some());
    }

    #[test]
    fn get_insert_zero() {
        let mut trie = Trie::new_temp();
        trie.insert(vec![0x0], b"value".to_vec()).unwrap();
        let first = trie.get(&[0x0][..].to_vec()).unwrap();
        assert_eq!(first, Some(b"value".to_vec()));
    }

    #[test]
    fn get_insert_a() {
        let mut trie = Trie::new_temp();
        trie.insert(vec![16], vec![0]).unwrap();
        trie.insert(vec![16, 0], vec![0]).unwrap();

        let item = trie.get(&vec![16]).unwrap();
        assert!(item.is_some());
        assert_eq!(item.unwrap(), vec![0]);

        let item = trie.get(&vec![16, 0]).unwrap();
        assert!(item.is_some());
        assert_eq!(item.unwrap(), vec![0]);
    }

    #[test]
    fn get_insert_b() {
        let mut trie = Trie::new_temp();
        trie.insert(vec![0, 0], vec![0, 0]).unwrap();
        trie.insert(vec![1, 0], vec![1, 0]).unwrap();

        let item = trie.get(&vec![1, 0]).unwrap();
        assert!(item.is_some());
        assert_eq!(item.unwrap(), vec![1, 0]);

        let item = trie.get(&vec![0, 0]).unwrap();
        assert!(item.is_some());
        assert_eq!(item.unwrap(), vec![0, 0]);
    }

    #[test]
    fn get_insert_c() {
        let mut trie = Trie::new_temp();
        trie.insert(vec![26, 192, 44, 251], vec![26, 192, 44, 251])
            .unwrap();
        trie.insert(
            vec![195, 132, 220, 124, 112, 201, 70, 128, 235],
            vec![195, 132, 220, 124, 112, 201, 70, 128, 235],
        )
        .unwrap();
        trie.insert(vec![126, 138, 25, 245, 146], vec![126, 138, 25, 245, 146])
            .unwrap();
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
        assert!(item.is_some());
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

    #[test]
    fn get_insert_d() {
        let mut trie = Trie::new_temp();
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

    #[test]
    fn get_insert_e() {
        let mut trie = Trie::new_temp();
        trie.insert(vec![0x00], vec![0x00]).unwrap();
        trie.insert(vec![0xC8], vec![0xC8]).unwrap();
        trie.insert(vec![0xC8, 0x00], vec![0xC8, 0x00]).unwrap();

        assert_eq!(trie.get(&vec![0x00]).unwrap(), Some(vec![0x00]));
        assert_eq!(trie.get(&vec![0xC8]).unwrap(), Some(vec![0xC8]));
        assert_eq!(trie.get(&vec![0xC8, 0x00]).unwrap(), Some(vec![0xC8, 0x00]));
    }

    #[test]
    fn get_insert_f() {
        let mut trie = Trie::new_temp();
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

    #[test]
    fn get_insert_remove_a() {
        let mut trie = Trie::new_temp();
        trie.insert("do".as_bytes().to_vec(), "verb".as_bytes().to_vec())
            .unwrap();
        trie.insert("horse".as_bytes().to_vec(), "stallion".as_bytes().to_vec())
            .unwrap();
        trie.insert("doge".as_bytes().to_vec(), "coin".as_bytes().to_vec())
            .unwrap();
        trie.remove("horse".as_bytes().to_vec()).unwrap();
        assert_eq!(
            trie.get(&"do".as_bytes().to_vec()).unwrap(),
            Some("verb".as_bytes().to_vec())
        );
    }

    #[test]
    fn get_insert_remove_b() {
        let mut trie = Trie::new_temp();
        trie.insert(vec![185], vec![185]).unwrap();
        trie.insert(vec![185, 0], vec![185, 0]).unwrap();
        trie.insert(vec![185, 1], vec![185, 1]).unwrap();
        trie.remove(vec![185, 1]).unwrap();
        assert_eq!(trie.get(&vec![185, 0]).unwrap(), Some(vec![185, 0]));
        assert_eq!(trie.get(&vec![185]).unwrap(), Some(vec![185]));
        assert!(trie.get(&vec![185, 1]).unwrap().is_none());
    }

    #[test]
    fn compute_hash_a() {
        let mut trie = Trie::new_temp();
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

    #[test]
    fn compute_hash_b() {
        let mut trie = Trie::new_temp();
        assert_eq!(
            trie.compute_hash().unwrap().as_slice(),
            hex::decode("56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421")
                .unwrap()
                .as_slice(),
        );
    }

    #[test]
    fn compute_hash_c() {
        let mut trie = Trie::new_temp();
        trie.insert(
            hex::decode("0000000000000000000000000000000000000000000000000000000000000045")
                .unwrap(),
            hex::decode("22b224a1420a802ab51d326e29fa98e34c4f24ea").unwrap(),
        )
        .unwrap();
        trie.insert(
            hex::decode("0000000000000000000000000000000000000000000000000000000000000046")
                .unwrap(),
            hex::decode("67706c2076330000000000000000000000000000000000000000000000000000")
                .unwrap(),
        )
        .unwrap();
        trie.insert(
            hex::decode("000000000000000000000000697c7b8c961b56f675d570498424ac8de1a918f6")
                .unwrap(),
            hex::decode("1234567890").unwrap(),
        )
        .unwrap();
        trie.insert(
            hex::decode("0000000000000000000000007ef9e639e2733cb34e4dfc576d4b23f72db776b2")
                .unwrap(),
            hex::decode("4655474156000000000000000000000000000000000000000000000000000000")
                .unwrap(),
        )
        .unwrap();
        trie.insert(
            hex::decode("000000000000000000000000ec4f34c97e43fbb2816cfd95e388353c7181dab1")
                .unwrap(),
            hex::decode("4e616d6552656700000000000000000000000000000000000000000000000000")
                .unwrap(),
        )
        .unwrap();
        trie.insert(
            hex::decode("4655474156000000000000000000000000000000000000000000000000000000")
                .unwrap(),
            hex::decode("7ef9e639e2733cb34e4dfc576d4b23f72db776b2").unwrap(),
        )
        .unwrap();
        trie.insert(
            hex::decode("4e616d6552656700000000000000000000000000000000000000000000000000")
                .unwrap(),
            hex::decode("ec4f34c97e43fbb2816cfd95e388353c7181dab1").unwrap(),
        )
        .unwrap();
        trie.insert(
            hex::decode("000000000000000000000000697c7b8c961b56f675d570498424ac8de1a918f6")
                .unwrap(),
            hex::decode("6f6f6f6820736f2067726561742c207265616c6c6c793f000000000000000000")
                .unwrap(),
        )
        .unwrap();
        trie.insert(
            hex::decode("6f6f6f6820736f2067726561742c207265616c6c6c793f000000000000000000")
                .unwrap(),
            hex::decode("697c7b8c961b56f675d570498424ac8de1a918f6").unwrap(),
        )
        .unwrap();

        assert_eq!(
            trie.compute_hash().unwrap().as_slice(),
            hex::decode("9f6221ebb8efe7cff60a716ecb886e67dd042014be444669f0159d8e68b42100")
                .unwrap()
                .as_slice(),
        );
    }

    #[test]
    fn compute_hash_d() {
        let mut trie = Trie::new_temp();
        trie.insert(
            "key1aa".as_bytes().to_vec(),
            "0123456789012345678901234567890123456789xxx"
                .as_bytes()
                .to_vec(),
        )
        .unwrap();
        trie.insert(
            "key1".as_bytes().to_vec(),
            "0123456789012345678901234567890123456789Very_Long"
                .as_bytes()
                .to_vec(),
        )
        .unwrap();
        trie.insert("key2bb".as_bytes().to_vec(), "aval3".as_bytes().to_vec())
            .unwrap();
        trie.insert("key2".as_bytes().to_vec(), "short".as_bytes().to_vec())
            .unwrap();
        trie.insert("key3cc".as_bytes().to_vec(), "aval3".as_bytes().to_vec())
            .unwrap();
        trie.insert(
            "key3".as_bytes().to_vec(),
            "1234567890123456789012345678901".as_bytes().to_vec(),
        )
        .unwrap();

        assert_eq!(
            trie.compute_hash().unwrap().as_slice(),
            hex::decode("cb65032e2f76c48b82b5c24b3db8f670ce73982869d38cd39a624f23d62a9e89")
                .unwrap()
                .as_slice(),
        );
    }

    #[test]
    fn compute_hash_e() {
        let mut trie = Trie::new_temp();
        trie.insert("abc".as_bytes().to_vec(), "123".as_bytes().to_vec())
            .unwrap();
        trie.insert("abcd".as_bytes().to_vec(), "abcd".as_bytes().to_vec())
            .unwrap();
        trie.insert("abc".as_bytes().to_vec(), "abc".as_bytes().to_vec())
            .unwrap();

        assert_eq!(
            trie.compute_hash().unwrap().as_slice(),
            hex::decode("7a320748f780ad9ad5b0837302075ce0eeba6c26e3d8562c67ccc0f1b273298a")
                .unwrap()
                .as_slice(),
        );
    }

    // Proptests
    proptest! {
        #[test]
        fn proptest_get_insert(data in btree_set(vec(any::<u8>(), 1..100), 1..100)) {
            let mut trie = Trie::new_temp();

            for val in data.iter(){
                trie.insert(val.clone(), val.clone()).unwrap();
            }

            for val in data.iter() {
                let item = trie.get(val).unwrap();
                prop_assert!(item.is_some());
                prop_assert_eq!(&item.unwrap(), val);
            }
        }

        #[test]
        fn proptest_get_insert_with_removals(mut data in vec((vec(any::<u8>(), 5..100), any::<bool>()), 1..100)) {
            let mut trie = Trie::new_temp();
            // Remove duplicate values with different expected status
            data.sort_by_key(|(val, _)| val.clone());
            data.dedup_by_key(|(val, _)| val.clone());
            // Insertions
            for (val, _) in data.iter() {
                trie.insert(val.clone(), val.clone()).unwrap();
            }
            // Removals
            for (val, should_remove) in data.iter() {
                if *should_remove {
                    let removed = trie.remove(val.clone()).unwrap();
                    prop_assert_eq!(removed, Some(val.clone()));
                }
            }
            // Check trie values
            for (val, removed) in data.iter() {
                let item = trie.get(val).unwrap();
                if !removed {
                    prop_assert_eq!(item, Some(val.clone()));
                } else {
                    prop_assert!(item.is_none());
                }
            }
        }

        #[test]
        fn proptest_compare_hash(data in btree_set(vec(any::<u8>(), 1..100), 1..100)) {
            let mut trie = Trie::new_temp();
            let mut cita_trie = cita_trie();

            for val in data.iter(){
                trie.insert(val.clone(), val.clone()).unwrap();
                cita_trie.insert(val.clone(), val.clone()).unwrap();
            }

            let hash = trie.compute_hash().unwrap().to_vec();
            let cita_hash = cita_trie.root().unwrap();
            prop_assert_eq!(hash, cita_hash);
        }

        #[test]
        fn proptest_compare_hash_with_removals(mut data in vec((vec(any::<u8>(), 5..100), any::<bool>()), 1..100)) {
            let mut trie = Trie::new_temp();
            let mut cita_trie = cita_trie();
            // Remove duplicate values with different expected status
            data.sort_by_key(|(val, _)| val.clone());
            data.dedup_by_key(|(val, _)| val.clone());
            // Insertions
            for (val, _) in data.iter() {
                trie.insert(val.clone(), val.clone()).unwrap();
                cita_trie.insert(val.clone(), val.clone()).unwrap();
            }
            // Removals
            for (val, should_remove) in data.iter() {
                if *should_remove {
                    trie.remove(val.clone()).unwrap();
                    cita_trie.remove(&val).unwrap();
                }
            }
            // Compare hashes
            let hash = trie.compute_hash().unwrap().to_vec();
            let cita_hash = cita_trie.root().unwrap();
            prop_assert_eq!(hash, cita_hash);
        }
    }

    fn cita_trie() -> CitaTrie<CitaMemoryDB, HasherKeccak> {
        let memdb = Arc::new(CitaMemoryDB::new(true));
        let hasher = Arc::new(HasherKeccak::new());

        CitaTrie::new(Arc::clone(&memdb), Arc::clone(&hasher))
    }
}
