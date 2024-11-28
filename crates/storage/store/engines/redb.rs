use std::{borrow::Borrow, ops::Mul, panic::RefUnwindSafe, sync::Arc};

use ethrex_core::{
    types::{BlockHash, BlockNumber, Index},
    H256,
};
use ethrex_trie::{db::redb::RedBTrie, Trie};
use redb::{AccessGuard, Database, Key, MultimapTableDefinition, TableDefinition, Value};

use crate::{
    error::StoreError,
    rlp::{AccountCodeHashRLP, AccountCodeRLP, BlockHashRLP, BlockHeaderRLP, ReceiptRLP, TupleRLP},
};

use super::api::StoreEngine;

const STATE_TRIE_NODES_TABLE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("StateTrieNodes");
const BLOCK_NUMBERS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("BlockNumbers");
const BLOCK_TOTAL_DIFFICULTIES_TABLE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("BlockTotalDifficulties");
const HEADERS_TABLE: TableDefinition<BlockHashRLP, BlockHeaderRLP> =
    TableDefinition::new("Headers");
const BODIES_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("Bodies");
const ACCOUNT_CODES_TABLE: TableDefinition<AccountCodeHashRLP, AccountCodeRLP> =
    TableDefinition::new("AccountCodes");
const RECEIPTS_TABLE: MultimapTableDefinition<TupleRLP<BlockHash, Index>, ReceiptRLP> =
    MultimapTableDefinition::new("Receipts");
const CANONICAL_BLOCK_HASHES_TABLE: TableDefinition<BlockNumber, BlockHashRLP> =
    TableDefinition::new("CanonicalBlockHashes");
const STORAGE_TRIE_NODES_TABLE: MultimapTableDefinition<([u8; 32], [u8; 32]), [u8; 32]> =
    MultimapTableDefinition::new("StorageTrieNodes");

// table_info!(BlockNumbers),
//         // TODO (#307): Remove TotalDifficulty.
//         table_info!(BlockTotalDifficulties),
//         table_info!(Headers),
//         table_info!(Bodies),
//         table_info!(AccountCodes),
//         table_info!(Receipts),
//         table_info!(TransactionLocations),
//         table_info!(ChainData),
//         table_info!(StateTrieNodes),
//         table_info!(StorageTriesNodes),
//         table_info!(CanonicalBlockHashes),
//         table_info!(Payloads),
//         table_info!(PendingBlocks),

#[derive(Debug)]
pub struct RedBStore {
    db: Arc<Database>,
}

impl RefUnwindSafe for RedBStore {}
impl RedBStore {
    pub fn new() -> Result<Self, StoreError> {
        Ok(Self {
            db: Arc::new(init_db()),
        })
    }

    // Helper method to write into a redb table
    fn write<'k, 'v, 'a, K, V>(
        &self,
        table: TableDefinition<'a, K, V>,
        key: impl Borrow<K::SelfType<'k>>,
        value: impl Borrow<V::SelfType<'v>>,
    ) -> Result<(), StoreError>
    where
        K: Key + 'static,
        V: Value + 'static,
    {
        let write_txn = self.db.begin_write().unwrap();
        {
            let mut table = write_txn.open_table(table).unwrap();
            table.insert(key, value).unwrap();
        }
        write_txn.commit().unwrap();

        Ok(())
    }

    // Helper method to read from a redb table
    fn read<'k, 'a, K, V>(
        &self,
        table: TableDefinition<'a, K, V>,
        key: impl Borrow<K::SelfType<'k>>,
    ) -> Result<Option<AccessGuard<'static, V>>, StoreError>
    where
        K: Key + 'static,
        V: Value,
    {
        let read_txn = self.db.begin_read().unwrap();
        let table = read_txn.open_table(table).unwrap();
        let result = match table.get(key).unwrap() {
            Some(value) => Some(value),
            None => None,
        };

        Ok(result)
    }

    fn get_block_hash_by_block_number(
        &self,
        number: BlockNumber,
    ) -> Result<Option<BlockHash>, StoreError> {
        Ok(self
            .read(CANONICAL_BLOCK_HASHES_TABLE, number)?
            .map(|a| a.value().to()))
    }
}

impl StoreEngine for RedBStore {
    fn add_block_header(
        &self,
        block_hash: ethrex_core::types::BlockHash,
        block_header: ethrex_core::types::BlockHeader,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_block_header(
        &self,
        block_number: ethrex_core::types::BlockNumber,
    ) -> Result<Option<ethrex_core::types::BlockHeader>, StoreError> {
        if let Some(hash) = self.get_block_hash_by_block_number(block_number)? {
            Ok(self
                .read(HEADERS_TABLE, <H256 as Into<BlockHashRLP>>::into(hash))?
                .map(|b| b.value().to()))
        } else {
            Ok(None)
        }
    }

    fn add_block_body(
        &self,
        block_hash: ethrex_core::types::BlockHash,
        block_body: ethrex_core::types::BlockBody,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_block_body(
        &self,
        block_number: ethrex_core::types::BlockNumber,
    ) -> Result<Option<ethrex_core::types::BlockBody>, StoreError> {
        todo!()
    }

    fn get_block_body_by_hash(
        &self,
        block_hash: ethrex_core::types::BlockHash,
    ) -> Result<Option<ethrex_core::types::BlockBody>, StoreError> {
        todo!()
    }

    fn get_block_header_by_hash(
        &self,
        block_hash: ethrex_core::types::BlockHash,
    ) -> Result<Option<ethrex_core::types::BlockHeader>, StoreError> {
        todo!()
    }

    fn add_pending_block(&self, block: ethrex_core::types::Block) -> Result<(), StoreError> {
        todo!()
    }

    fn get_pending_block(
        &self,
        block_hash: ethrex_core::types::BlockHash,
    ) -> Result<Option<ethrex_core::types::Block>, StoreError> {
        todo!()
    }

    fn add_block_number(
        &self,
        block_hash: ethrex_core::types::BlockHash,
        block_number: ethrex_core::types::BlockNumber,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_block_number(
        &self,
        block_hash: ethrex_core::types::BlockHash,
    ) -> Result<Option<ethrex_core::types::BlockNumber>, StoreError> {
        todo!()
    }

    fn add_block_total_difficulty(
        &self,
        block_hash: ethrex_core::types::BlockHash,
        block_total_difficulty: ethrex_core::U256,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_block_total_difficulty(
        &self,
        block_hash: ethrex_core::types::BlockHash,
    ) -> Result<Option<ethrex_core::U256>, StoreError> {
        todo!()
    }

    fn add_transaction_location(
        &self,
        transaction_hash: ethrex_core::H256,
        block_number: ethrex_core::types::BlockNumber,
        block_hash: ethrex_core::types::BlockHash,
        index: ethrex_core::types::Index,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_transaction_location(
        &self,
        transaction_hash: ethrex_core::H256,
    ) -> Result<
        Option<(
            ethrex_core::types::BlockNumber,
            ethrex_core::types::BlockHash,
            ethrex_core::types::Index,
        )>,
        StoreError,
    > {
        todo!()
    }

    fn add_receipt(
        &self,
        block_hash: ethrex_core::types::BlockHash,
        index: ethrex_core::types::Index,
        receipt: ethrex_core::types::Receipt,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_receipt(
        &self,
        block_number: ethrex_core::types::BlockNumber,
        index: ethrex_core::types::Index,
    ) -> Result<Option<ethrex_core::types::Receipt>, StoreError> {
        todo!()
    }

    fn add_account_code(
        &self,
        code_hash: ethrex_core::H256,
        code: bytes::Bytes,
    ) -> Result<(), StoreError> {
        self.write(
            ACCOUNT_CODES_TABLE,
            <H256 as Into<AccountCodeHashRLP>>::into(code_hash),
            <bytes::Bytes as Into<AccountCodeRLP>>::into(code.into()),
        )
    }

    fn get_account_code(
        &self,
        code_hash: ethrex_core::H256,
    ) -> Result<Option<bytes::Bytes>, StoreError> {
        todo!()
    }

    fn get_canonical_block_hash(
        &self,
        block_number: ethrex_core::types::BlockNumber,
    ) -> Result<Option<ethrex_core::types::BlockHash>, StoreError> {
        todo!()
    }

    fn set_chain_config(
        &self,
        chain_config: &ethrex_core::types::ChainConfig,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_chain_config(&self) -> Result<ethrex_core::types::ChainConfig, StoreError> {
        todo!()
    }

    fn update_earliest_block_number(
        &self,
        block_number: ethrex_core::types::BlockNumber,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_earliest_block_number(
        &self,
    ) -> Result<Option<ethrex_core::types::BlockNumber>, StoreError> {
        todo!()
    }

    fn update_finalized_block_number(
        &self,
        block_number: ethrex_core::types::BlockNumber,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_finalized_block_number(
        &self,
    ) -> Result<Option<ethrex_core::types::BlockNumber>, StoreError> {
        todo!()
    }

    fn update_safe_block_number(
        &self,
        block_number: ethrex_core::types::BlockNumber,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_safe_block_number(&self) -> Result<Option<ethrex_core::types::BlockNumber>, StoreError> {
        todo!()
    }

    fn update_latest_block_number(
        &self,
        block_number: ethrex_core::types::BlockNumber,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_latest_block_number(
        &self,
    ) -> Result<Option<ethrex_core::types::BlockNumber>, StoreError> {
        todo!()
    }

    fn update_latest_total_difficulty(
        &self,
        latest_total_difficulty: ethrex_core::U256,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_latest_total_difficulty(&self) -> Result<Option<ethrex_core::U256>, StoreError> {
        todo!()
    }

    fn update_pending_block_number(
        &self,
        block_number: ethrex_core::types::BlockNumber,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_pending_block_number(
        &self,
    ) -> Result<Option<ethrex_core::types::BlockNumber>, StoreError> {
        todo!()
    }

    fn open_storage_trie(
        &self,
        hashed_address: ethrex_core::H256,
        storage_root: ethrex_core::H256,
    ) -> ethrex_trie::Trie {
        // let db = Box::new(LibmdbxDupsortTrieDB::<StorageTriesNodes, [u8; 32]>::new(
        //     self.db.clone(),
        //     hashed_address.0,
        // ));
        // Trie::open(db, storage_root)
        // let db = Box::new(LibmdbxDupsortTrieDB::<StorageTriesNodes, [u8; 32]>::new(
        //     self.db.clone(),
        //     hashed_address.0,
        // ));
        // Trie::open(db, storage_root)
        todo!()
    }

    fn open_state_trie(&self, state_root: ethrex_core::H256) -> ethrex_trie::Trie {
        let db = Box::new(RedBTrie::new(self.db.clone()));
        Trie::open(db, state_root)
    }

    fn set_canonical_block(
        &self,
        number: ethrex_core::types::BlockNumber,
        hash: ethrex_core::types::BlockHash,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn unset_canonical_block(
        &self,
        number: ethrex_core::types::BlockNumber,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn add_payload(
        &self,
        payload_id: u64,
        block: ethrex_core::types::Block,
    ) -> Result<(), StoreError> {
        todo!()
    }

    fn get_payload(
        &self,
        payload_id: u64,
    ) -> Result<Option<ethrex_core::types::Block>, StoreError> {
        todo!()
    }
}

/// Initializes a new database with the provided path. If the path is `None`, the database
/// will be temporary.
pub fn init_db() -> Database {
    let db = Database::create("ethrex.redb").unwrap();

    let table_creation_txn = db.begin_write().unwrap();
    table_creation_txn
        .open_table(STATE_TRIE_NODES_TABLE)
        .unwrap();
    table_creation_txn.open_table(BLOCK_NUMBERS_TABLE).unwrap();
    table_creation_txn
        .open_table(BLOCK_TOTAL_DIFFICULTIES_TABLE)
        .unwrap();
    table_creation_txn
        .open_table(CANONICAL_BLOCK_HASHES_TABLE)
        .unwrap();
    table_creation_txn
        .open_multimap_table(RECEIPTS_TABLE)
        .unwrap();
    table_creation_txn
        .open_multimap_table(STORAGE_TRIE_NODES_TABLE)
        .unwrap();
    table_creation_txn.commit().unwrap();

    db
}
