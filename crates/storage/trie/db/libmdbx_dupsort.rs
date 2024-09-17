use std::{marker::PhantomData, sync::Arc};

use crate::error::StoreError;
use libmdbx::orm::{Database, DupSort, Table};

use super::TrieDB;

/// Libmdbx implementation for the TrieDB trait for a dupsort table with a fixed primary key.
/// For a dupsort table (A, B)[A] -> C, this trie will have a fixed A and just work on B -> C
/// A will be a fixed-size encoded key set by the user, B will be a fixed-size encoded NodeHash and C will be an encoded Node
pub struct LibmdbxDupsortTrieDB<T: Table + DupSort> {
    db: Arc<Database>,
    fixed_key: T::SeekKey,
    phantom: PhantomData<T>,
}

impl<T> LibmdbxDupsortTrieDB<T>
where
    T: DupSort<Key = ([u8; 32], [u8; 33]), SeekKey = [u8; 32], Value = Vec<u8>>,
{
    pub fn new(db: Arc<Database>, fixed_key: T::SeekKey) -> Self {
        Self {
            db,
            fixed_key,
            phantom: PhantomData,
        }
    }
}

impl<T> TrieDB for LibmdbxDupsortTrieDB<T>
where
    T: DupSort<Key = ([u8; 32], [u8; 33]), SeekKey = [u8; 32], Value = Vec<u8>>,
{
    fn get(&self, key: Vec<u8>) -> Result<Option<Vec<u8>>, StoreError> {
        let txn = self.db.begin_read().map_err(StoreError::LibmdbxError)?;
        txn.get::<T>((self.fixed_key, node_hash_to_fixed_size(key)))
            .map_err(StoreError::LibmdbxError)
    }

    fn put(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), StoreError> {
        let txn = self
            .db
            .begin_readwrite()
            .map_err(StoreError::LibmdbxError)?;
        txn.upsert::<T>((self.fixed_key, node_hash_to_fixed_size(key)), value)
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }
}

// In order to use NodeHash as key in a dupsort table we must encode it into a fixed size type
fn node_hash_to_fixed_size(node_hash: Vec<u8>) -> [u8; 33] {
    // keep original len so we can re-construct it later
    let original_len = node_hash.len();
    // original len will always be lower or equal to 32 bytes
    debug_assert!(original_len <= 32);
    // Pad the node_hash with zeros to make it fixed_size (in case of inline)
    let mut node_hash = node_hash;
    node_hash.resize(32, 0);
    // Encode the node as [original_len, node_hash...]
    std::array::from_fn(|i| match i {
        0 => original_len as u8,
        n => node_hash[n - 1],
    })
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::trie::test_utils::new_db;
    use libmdbx::{dupsort, table};

    dupsort!(
        /// (Key + NodeHash) to Node table
        ( Nodes )  ([u8;32], [u8;33])[[u8;32]] => Vec<u8>
    );

    #[test]
    fn simple_addition() {
        let inner_db = new_db::<Nodes>();
        let db = LibmdbxDupsortTrieDB::<Nodes>::new(inner_db, [5; 32]);
        assert_eq!(db.get("hello".into()).unwrap(), None);
        db.put("hello".into(), "value".into()).unwrap();
        assert_eq!(db.get("hello".into()).unwrap(), Some("value".into()));
    }

    #[test]
    fn different_keys() {
        let inner_db = new_db::<Nodes>();
        let db_a = LibmdbxDupsortTrieDB::<Nodes>::new(inner_db.clone(), [5; 32]);
        let db_b = LibmdbxDupsortTrieDB::<Nodes>::new(inner_db, [7; 32]);
        db_a.put("hello".into(), "hello!".into()).unwrap();
        db_b.put("hello".into(), "go away!".into()).unwrap();
        assert_eq!(db_a.get("hello".into()).unwrap(), Some("hello!".into()));
        assert_eq!(db_b.get("hello".into()).unwrap(), Some("go away!".into()));
    }
}
