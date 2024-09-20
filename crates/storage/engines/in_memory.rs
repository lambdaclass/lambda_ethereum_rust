use crate::{error::StoreError, trie::Trie};
use bytes::Bytes;
use ethereum_rust_core::types::{
    BlockBody, BlockHash, BlockHeader, BlockNumber, ChainConfig, Index, Receipt, Transaction,
};
use ethereum_types::{Address, H256};
use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, Mutex},
};

use super::api::StoreEngine;

pub type NodeMap = Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>;

#[derive(Default)]
pub struct Store {
    chain_data: ChainData,
    block_numbers: HashMap<BlockHash, BlockNumber>,
    canonical_hashes: HashMap<BlockNumber, BlockHash>,
    bodies: HashMap<BlockHash, BlockBody>,
    headers: HashMap<BlockHash, BlockHeader>,
    // Maps code hashes to code
    account_codes: HashMap<H256, Bytes>,
    // Maps transaction hashes to their blocks (height+hash) and index within the blocks.
    transaction_locations: HashMap<H256, Vec<(BlockNumber, BlockHash, Index)>>,
    // Stores pooled transactions by their hashes
    transaction_pool: HashMap<H256, Transaction>,
    receipts: HashMap<BlockHash, HashMap<Index, Receipt>>,
    state_trie_nodes: NodeMap,
    storage_trie_nodes: HashMap<Address, NodeMap>,
}

#[derive(Default)]
struct ChainData {
    chain_config: Option<ChainConfig>,
    earliest_block_number: Option<BlockNumber>,
    finalized_block_number: Option<BlockNumber>,
    safe_block_number: Option<BlockNumber>,
    latest_block_number: Option<BlockNumber>,
    pending_block_number: Option<BlockNumber>,
}

impl Store {
    pub fn new() -> Result<Self, StoreError> {
        Ok(Self::default())
    }
}

impl StoreEngine for Store {
    fn get_block_header(&self, block_number: u64) -> Result<Option<BlockHeader>, StoreError> {
        if let Some(hash) = self.canonical_hashes.get(&block_number) {
            Ok(self.headers.get(hash).cloned())
        } else {
            Ok(None)
        }
    }

    fn get_block_body(&self, block_number: u64) -> Result<Option<BlockBody>, StoreError> {
        if let Some(hash) = self.canonical_hashes.get(&block_number) {
            Ok(self.bodies.get(hash).cloned())
        } else {
            Ok(None)
        }
    }

    fn add_block_header(
        &mut self,
        block_hash: BlockHash,
        block_header: BlockHeader,
    ) -> Result<(), StoreError> {
        self.headers.insert(block_hash, block_header);
        Ok(())
    }

    fn add_block_body(
        &mut self,
        block_hash: BlockHash,
        block_body: BlockBody,
    ) -> Result<(), StoreError> {
        self.bodies.insert(block_hash, block_body);
        Ok(())
    }

    fn add_block_number(
        &mut self,
        block_hash: BlockHash,
        block_number: BlockNumber,
    ) -> Result<(), StoreError> {
        self.block_numbers.insert(block_hash, block_number);
        Ok(())
    }

    fn get_block_number(&self, block_hash: BlockHash) -> Result<Option<BlockNumber>, StoreError> {
        Ok(self.block_numbers.get(&block_hash).copied())
    }

    fn add_transaction_location(
        &mut self,
        transaction_hash: H256,
        block_number: BlockNumber,
        block_hash: BlockHash,
        index: Index,
    ) -> Result<(), StoreError> {
        self.transaction_locations
            .entry(transaction_hash)
            .or_default()
            .push((block_number, block_hash, index));
        Ok(())
    }

    fn get_transaction_location(
        &self,
        transaction_hash: H256,
    ) -> Result<Option<(BlockNumber, BlockHash, Index)>, StoreError> {
        Ok(self
            .transaction_locations
            .get(&transaction_hash)
            .and_then(|v| {
                v.iter()
                    .find(|(number, hash, _index)| self.canonical_hashes.get(number) == Some(hash))
                    .copied()
            }))
    }

    fn add_transaction_to_pool(
        &mut self,
        hash: H256,
        transaction: Transaction,
    ) -> Result<(), StoreError> {
        self.transaction_pool.insert(hash, transaction);
        Ok(())
    }

    fn get_transaction_from_pool(&self, hash: H256) -> Result<Option<Transaction>, StoreError> {
        Ok(self.transaction_pool.get(&hash).cloned())
    }

    fn add_receipt(
        &mut self,
        block_hash: BlockHash,
        index: Index,
        receipt: Receipt,
    ) -> Result<(), StoreError> {
        let entry = self.receipts.entry(block_hash).or_default();
        entry.insert(index, receipt);
        Ok(())
    }

    fn get_receipt(
        &self,
        block_number: BlockNumber,
        index: Index,
    ) -> Result<Option<Receipt>, StoreError> {
        if let Some(hash) = self.canonical_hashes.get(&block_number) {
            Ok(self
                .receipts
                .get(hash)
                .and_then(|entry| entry.get(&index))
                .cloned())
        } else {
            Ok(None)
        }
    }

    fn add_account_code(&mut self, code_hash: H256, code: Bytes) -> Result<(), StoreError> {
        self.account_codes.insert(code_hash, code);
        Ok(())
    }

    fn get_account_code(&self, code_hash: H256) -> Result<Option<Bytes>, StoreError> {
        Ok(self.account_codes.get(&code_hash).cloned())
    }

    fn set_chain_config(&mut self, chain_config: &ChainConfig) -> Result<(), StoreError> {
        // Store cancun timestamp
        self.chain_data.chain_config = Some(*chain_config);
        Ok(())
    }

    fn get_chain_config(&self) -> Result<ChainConfig, StoreError> {
        Ok(self.chain_data.chain_config.unwrap())
    }

    fn update_earliest_block_number(
        &mut self,
        block_number: BlockNumber,
    ) -> Result<(), StoreError> {
        self.chain_data.earliest_block_number.replace(block_number);
        Ok(())
    }

    fn get_earliest_block_number(&self) -> Result<Option<BlockNumber>, StoreError> {
        Ok(self.chain_data.earliest_block_number)
    }

    fn update_finalized_block_number(
        &mut self,
        block_number: BlockNumber,
    ) -> Result<(), StoreError> {
        self.chain_data.finalized_block_number.replace(block_number);
        Ok(())
    }

    fn get_finalized_block_number(&self) -> Result<Option<BlockNumber>, StoreError> {
        Ok(self.chain_data.finalized_block_number)
    }

    fn update_safe_block_number(&mut self, block_number: BlockNumber) -> Result<(), StoreError> {
        self.chain_data.safe_block_number.replace(block_number);
        Ok(())
    }

    fn get_safe_block_number(&self) -> Result<Option<BlockNumber>, StoreError> {
        Ok(self.chain_data.safe_block_number)
    }

    fn update_latest_block_number(&mut self, block_number: BlockNumber) -> Result<(), StoreError> {
        self.chain_data.latest_block_number.replace(block_number);
        Ok(())
    }

    fn get_latest_block_number(&self) -> Result<Option<BlockNumber>, StoreError> {
        Ok(self.chain_data.latest_block_number)
    }

    fn update_pending_block_number(&mut self, block_number: BlockNumber) -> Result<(), StoreError> {
        self.chain_data.pending_block_number.replace(block_number);
        Ok(())
    }

    fn get_pending_block_number(&self) -> Result<Option<BlockNumber>, StoreError> {
        Ok(self.chain_data.pending_block_number)
    }

    fn state_trie(&self, block_number: BlockNumber) -> Result<Option<Trie>, StoreError> {
        let Some(state_root) = self.get_block_header(block_number)?.map(|h| h.state_root) else {
            return Ok(None);
        };
        let db = Box::new(crate::trie::InMemoryTrieDB::new(
            self.state_trie_nodes.clone(),
        ));
        let trie = Trie::open(db, state_root);
        Ok(Some(trie))
    }

    fn new_state_trie(&self) -> Result<Trie, StoreError> {
        let db = Box::new(crate::trie::InMemoryTrieDB::new(
            self.state_trie_nodes.clone(),
        ));
        let trie = Trie::new(db);
        Ok(trie)
    }

    fn open_storage_trie(&mut self, address: Address, storage_root: H256) -> Trie {
        let trie_backend = self.storage_trie_nodes.entry(address).or_default();
        let db = Box::new(crate::trie::InMemoryTrieDB::new(trie_backend.clone()));
        Trie::open(db, storage_root)
    }

    fn get_block_body_by_hash(
        &self,
        block_hash: BlockHash,
    ) -> Result<Option<BlockBody>, StoreError> {
        Ok(self.bodies.get(&block_hash).cloned())
    }

    fn get_block_header_by_hash(
        &self,
        block_hash: BlockHash,
    ) -> Result<Option<BlockHeader>, StoreError> {
        Ok(self.headers.get(&block_hash).cloned())
    }

    fn set_canonical_block(
        &mut self,
        number: BlockNumber,
        hash: BlockHash,
    ) -> Result<(), StoreError> {
        self.canonical_hashes.insert(number, hash);
        Ok(())
    }

    fn get_canonical_block_hash(
        &self,
        block_number: BlockNumber,
    ) -> Result<Option<BlockHash>, StoreError> {
        Ok(self.canonical_hashes.get(&block_number).cloned())
    }
}

impl Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("In Memory Store").finish()
    }
}
