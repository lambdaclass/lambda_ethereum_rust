use bytes::Bytes;
use ethereum_rust_core::types::{
    Block, BlockBody, BlockHash, BlockHeader, BlockNumber, ChainConfig, Index, Receipt, Transaction,
};
use ethereum_types::{Address, H256, U256};
use std::fmt::Debug;

use crate::{error::StoreError, trie::Trie};

pub trait StoreEngine: Debug + Send {
    /// Add block header
    fn add_block_header(
        &mut self,
        block_hash: BlockHash,
        block_header: BlockHeader,
    ) -> Result<(), StoreError>;

    /// Obtain canonical block header
    fn get_block_header(
        &self,
        block_number: BlockNumber,
    ) -> Result<Option<BlockHeader>, StoreError>;

    /// Add block body
    fn add_block_body(
        &mut self,
        block_hash: BlockHash,
        block_body: BlockBody,
    ) -> Result<(), StoreError>;

    /// Obtain canonical block body
    fn get_block_body(&self, block_number: BlockNumber) -> Result<Option<BlockBody>, StoreError>;

    /// Obtain any block body using the hash
    fn get_block_body_by_hash(
        &self,
        block_hash: BlockHash,
    ) -> Result<Option<BlockBody>, StoreError>;

    fn get_block_header_by_hash(
        &self,
        block_hash: BlockHash,
    ) -> Result<Option<BlockHeader>, StoreError>;

    /// Add block number for a given hash
    fn add_block_number(
        &mut self,
        block_hash: BlockHash,
        block_number: BlockNumber,
    ) -> Result<(), StoreError>;

    /// Obtain block number for a given hash
    fn get_block_number(&self, block_hash: BlockHash) -> Result<Option<BlockNumber>, StoreError>;

    /// Store transaction location (block number and index of the transaction within the block)
    fn add_transaction_location(
        &mut self,
        transaction_hash: H256,
        block_number: BlockNumber,
        block_hash: BlockHash,
        index: Index,
    ) -> Result<(), StoreError>;

    /// Obtain transaction location (block hash and index)
    fn get_transaction_location(
        &self,
        transaction_hash: H256,
    ) -> Result<Option<(BlockNumber, BlockHash, Index)>, StoreError>;

    /// Add receipt
    fn add_receipt(
        &mut self,
        block_hash: BlockHash,
        index: Index,
        receipt: Receipt,
    ) -> Result<(), StoreError>;

    /// Obtain receipt for a canonical block represented by the block number.
    fn get_receipt(
        &self,
        block_number: BlockNumber,
        index: Index,
    ) -> Result<Option<Receipt>, StoreError>;

    /// Add account code
    fn add_account_code(&mut self, code_hash: H256, code: Bytes) -> Result<(), StoreError>;

    /// Obtain account code via code hash
    fn get_account_code(&self, code_hash: H256) -> Result<Option<Bytes>, StoreError>;

    fn get_transaction_by_hash(
        &self,
        transaction_hash: H256,
    ) -> Result<Option<Transaction>, StoreError> {
        let (_block_number, block_hash, index) =
            match self.get_transaction_location(transaction_hash)? {
                Some(location) => location,
                None => return Ok(None),
            };
        self.get_transaction_by_location(block_hash, index)
    }

    fn get_transaction_by_location(
        &self,
        block_hash: H256,
        index: u64,
    ) -> Result<Option<Transaction>, StoreError> {
        let block_body = match self.get_block_body_by_hash(block_hash)? {
            Some(body) => body,
            None => return Ok(None),
        };
        Ok(index
            .try_into()
            .ok()
            .and_then(|index: usize| block_body.transactions.get(index).cloned()))
    }

    fn get_block_by_hash(&self, block_hash: BlockHash) -> Result<Option<Block>, StoreError> {
        let header = match self.get_block_header_by_hash(block_hash)? {
            Some(header) => header,
            None => return Ok(None),
        };
        let body = match self.get_block_body_by_hash(block_hash)? {
            Some(body) => body,
            None => return Ok(None),
        };
        Ok(Some(Block { header, body }))
    }

    // Add storage value
    fn add_storage_at(
        &mut self,
        address: Address,
        storage_key: H256,
        storage_value: U256,
    ) -> Result<(), StoreError>;

    // Obtain storage value
    fn get_storage_at(
        &self,
        address: Address,
        storage_key: H256,
    ) -> Result<Option<U256>, StoreError>;

    // Add storage value
    fn remove_account_storage(&mut self, address: Address) -> Result<(), StoreError>;

    // Get full account storage
    fn account_storage_iter(
        &mut self,
        address: Address,
    ) -> Result<Box<dyn Iterator<Item = (H256, U256)>>, StoreError>;

    /// Stores the chain configuration values, should only be called once after reading the genesis file
    /// Ignores previously stored values if present
    fn set_chain_config(&mut self, chain_config: &ChainConfig) -> Result<(), StoreError>;

    /// Returns the stored chain configuration
    fn get_chain_config(&self) -> Result<ChainConfig, StoreError>;

    // Update earliest block number
    fn update_earliest_block_number(&mut self, block_number: BlockNumber)
        -> Result<(), StoreError>;

    // Obtain earliest block number
    fn get_earliest_block_number(&self) -> Result<Option<BlockNumber>, StoreError>;

    // Update finalized block number
    fn update_finalized_block_number(
        &mut self,
        block_number: BlockNumber,
    ) -> Result<(), StoreError>;

    // Obtain finalized block number
    fn get_finalized_block_number(&self) -> Result<Option<BlockNumber>, StoreError>;

    // Update safe block number
    fn update_safe_block_number(&mut self, block_number: BlockNumber) -> Result<(), StoreError>;

    // Obtain safe block number
    fn get_safe_block_number(&self) -> Result<Option<BlockNumber>, StoreError>;

    // Update latest block number
    fn update_latest_block_number(&mut self, block_number: BlockNumber) -> Result<(), StoreError>;

    // Obtain latest block number
    fn get_latest_block_number(&self) -> Result<Option<BlockNumber>, StoreError>;

    // Update pending block number
    fn update_pending_block_number(&mut self, block_number: BlockNumber) -> Result<(), StoreError>;

    // Obtain pending block number
    fn get_pending_block_number(&self) -> Result<Option<BlockNumber>, StoreError>;

    // Obtain the world state trie for the given block
    fn state_trie(&self, block_number: BlockNumber) -> Result<Option<Trie>, StoreError>;

    // Obtain a world state from an empty root
    // This method should be used when creating the genesis world state
    fn new_state_trie(&self) -> Result<Trie, StoreError>;

    // Get the canonical block hash for a given block number.
    fn set_canonical_block(
        &mut self,
        number: BlockNumber,
        hash: BlockHash,
    ) -> Result<(), StoreError>;
}
