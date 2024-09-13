use std::marker::PhantomData;

use crate::error::StoreError;
use libmdbx::{
    orm::{table, Database, Table},
    table_info,
};

use crate::trie::db::{NodeHashRLP, NodeRLP};

/// Libmdbx implementation for the TrieDB trait, with get and put operations.
pub struct Libmdbx<'a, T: Table> {
    db: &'a Database,
    phantom: PhantomData<T>,
}

use super::TrieDB;

table!(
    /// NodeHash to Node table
    ( Nodes )  NodeHashRLP => NodeRLP
);

impl<'a, T: Table> TrieDB for Libmdbx<'a, T> {
    fn get(&self, key: NodeHashRLP) -> Result<Option<NodeRLP>, StoreError> {
        let txn = self.db.begin_read().map_err(StoreError::LibmdbxError)?;
        txn.get::<T>(key).map_err(StoreError::LibmdbxError)
    }

    fn put(&self, key: NodeHashRLP, value: NodeRLP) -> Result<(), StoreError> {
        let txn = self
            .db
            .begin_readwrite()
            .map_err(StoreError::LibmdbxError)?;
        txn.upsert::<T>(key, value)
            .map_err(StoreError::LibmdbxError)?;
        txn.commit().map_err(StoreError::LibmdbxError)
    }
}

#[cfg(test)]
fn new_db() -> Database {
    let tables = [table_info!(Nodes)].into_iter().collect();
    Database::create(None, &tables).expect("Failed to create temp DB")
}

#[test]
fn simple_addition() {
    let inner_db = new_db();
    let db = Libmdbx::<Nodes> { db: inner_db };
    assert_eq!(db.get("hello".into()).unwrap(), None);
    db.put("hello".into(), "value".into());
    assert_eq!(db.get("hello".into()).unwrap(), Some("value".into()));
}

#[test]
fn different_tables() {
    table!(
        /// vec to vec
        ( TableA ) Vec<u8> => Vec<u8>
    );
    table!(
        /// vec to vec
        ( TableB ) Vec<u8> => Vec<u8>
    );
    let inner_db = new_db();
    let tables = [table_info!(TableA), table_info!(TableB)]
        .into_iter()
        .collect();

    let db = Database::create(None, &tables).unwrap();
    let trie_dba = Libmdbx::<TableA> {
        db: &db,
        phantom: PhantomData,
    };
    let trie_dbb = Libmdbx::<TableB> {
        db: &db,
        phantom: PhantomData,
    };
    trie_dba.put("hello".into(), "value".into());
    assert_eq!(trie_dbb.get("hello".into()).unwrap(), Some("value".into()));
}
