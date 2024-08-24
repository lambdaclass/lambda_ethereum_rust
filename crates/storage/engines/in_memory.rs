use crate::error::StoreError;
use bytes::Bytes;
use ethereum_rust_core::types::{
    AccountInfo, BlockBody, BlockHash, BlockHeader, BlockNumber, ChainConfig, Index, Receipt,
};
use ethereum_types::{Address, H256, U256};
use std::{collections::HashMap, fmt::Debug};

use super::api::StoreEngine;

#[derive(Default)]
pub struct Store {
    chain_data: ChainData,
    account_infos: HashMap<Address, AccountInfo>,
    block_numbers: HashMap<BlockHash, BlockNumber>,
    block_total_difficulties: HashMap<BlockHash, U256>,
    bodies: HashMap<BlockNumber, BlockBody>,
    headers: HashMap<BlockNumber, BlockHeader>,
    // Maps code hashes to code
    account_codes: HashMap<H256, Bytes>,
    account_storages: HashMap<Address, HashMap<H256, U256>>,
    // Maps transaction hashes to their block number and index within the block
    transaction_locations: HashMap<H256, (BlockNumber, Index)>,
    receipts: HashMap<BlockNumber, HashMap<Index, Receipt>>,
}

#[derive(Default)]
struct ChainData {
    chain_id: Option<U256>,
    earliest_block_number: Option<BlockNumber>,
    finalized_block_number: Option<BlockNumber>,
    safe_block_number: Option<BlockNumber>,
    latest_block_number: Option<BlockNumber>,
    latest_total_difficulty: Option<U256>,
    pending_block_number: Option<BlockNumber>,
    cancun_time: Option<u64>,
    shanghai_time: Option<u64>,
}

impl Store {
    pub fn new() -> Result<Self, StoreError> {
        Ok(Self::default())
    }
}

impl StoreEngine for Store {
    fn add_account_info(
        &mut self,
        address: Address,
        account_info: AccountInfo,
    ) -> Result<(), StoreError> {
        self.account_infos.insert(address, account_info);
        Ok(())
    }

    fn get_account_info(&self, address: Address) -> Result<Option<AccountInfo>, StoreError> {
        Ok(self.account_infos.get(&address).cloned())
    }

    fn remove_account_info(&mut self, address: Address) -> Result<(), StoreError> {
        self.account_infos.remove(&address);
        Ok(())
    }

    fn account_infos_iter(
        &self,
    ) -> Result<Box<dyn Iterator<Item = (Address, AccountInfo)>>, StoreError> {
        Ok(Box::new(self.account_infos.clone().into_iter()))
    }

    fn get_block_header(&self, block_number: u64) -> Result<Option<BlockHeader>, StoreError> {
        Ok(self.headers.get(&block_number).cloned())
    }

    fn get_block_body(&self, block_number: u64) -> Result<Option<BlockBody>, StoreError> {
        Ok(self.bodies.get(&block_number).cloned())
    }

    fn add_block_header(
        &mut self,
        block_number: BlockNumber,
        block_header: BlockHeader,
    ) -> Result<(), StoreError> {
        self.headers.insert(block_number, block_header);
        Ok(())
    }

    fn add_block_body(
        &mut self,
        block_number: BlockNumber,
        block_body: BlockBody,
    ) -> Result<(), StoreError> {
        self.bodies.insert(block_number, block_body);
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

    fn add_block_total_difficulty(
        &mut self,
        block_hash: BlockHash,
        block_total_difficulty: U256,
    ) -> Result<(), StoreError> {
        self.block_total_difficulties
            .insert(block_hash, block_total_difficulty);
        Ok(())
    }

    fn get_block_total_difficulty(
        &self,
        block_hash: BlockHash,
    ) -> Result<Option<U256>, StoreError> {
        Ok(self.block_total_difficulties.get(&block_hash).copied())
    }

    fn add_transaction_location(
        &mut self,
        transaction_hash: H256,
        block_number: BlockNumber,
        index: Index,
    ) -> Result<(), StoreError> {
        self.transaction_locations
            .insert(transaction_hash, (block_number, index));
        Ok(())
    }

    fn get_transaction_location(
        &self,
        transaction_hash: H256,
    ) -> Result<Option<(BlockNumber, Index)>, StoreError> {
        Ok(self.transaction_locations.get(&transaction_hash).copied())
    }

    fn add_receipt(
        &mut self,
        block_number: BlockNumber,
        index: Index,
        receipt: Receipt,
    ) -> Result<(), StoreError> {
        let entry = self.receipts.entry(block_number).or_default();
        entry.insert(index, receipt);
        Ok(())
    }

    fn get_receipt(
        &self,
        block_number: BlockNumber,
        index: Index,
    ) -> Result<Option<Receipt>, StoreError> {
        Ok(self
            .receipts
            .get(&block_number)
            .and_then(|entry| entry.get(&index))
            .cloned())
    }

    fn add_account_code(&mut self, code_hash: H256, code: Bytes) -> Result<(), StoreError> {
        self.account_codes.insert(code_hash, code);
        Ok(())
    }

    fn get_account_code(&self, code_hash: H256) -> Result<Option<Bytes>, StoreError> {
        Ok(self.account_codes.get(&code_hash).cloned())
    }

    fn add_storage_at(
        &mut self,
        address: Address,
        storage_key: H256,
        storage_value: U256,
    ) -> Result<(), StoreError> {
        let entry = self.account_storages.entry(address).or_default();
        entry.insert(storage_key, storage_value);
        Ok(())
    }

    fn get_storage_at(
        &self,
        address: Address,
        storage_key: H256,
    ) -> Result<Option<U256>, StoreError> {
        Ok(self
            .account_storages
            .get(&address)
            .and_then(|entry| entry.get(&storage_key).cloned()))
    }

    fn remove_account_storage(&mut self, address: Address) -> Result<(), StoreError> {
        self.account_storages.remove(&address);
        Ok(())
    }

    fn account_storage_iter(
        &mut self,
        address: Address,
    ) -> Result<Box<dyn Iterator<Item = (H256, U256)>>, StoreError> {
        Ok(Box::new(
            self.account_storages
                .get(&address)
                .cloned()
                .into_iter()
                .flatten(),
        ))
    }

    fn set_chain_config(&mut self, chain_config: &ChainConfig) -> Result<(), StoreError> {
        // Store cancun timestamp
        self.chain_data.cancun_time = chain_config.cancun_time;
        // Store shanghai timestamp
        self.chain_data.shanghai_time = chain_config.shanghai_time;
        // Store chain id
        self.chain_data.chain_id.replace(chain_config.chain_id);
        Ok(())
    }

    fn get_chain_id(&self) -> Result<Option<U256>, StoreError> {
        Ok(self.chain_data.chain_id)
    }

    fn get_cancun_time(&self) -> Result<Option<u64>, StoreError> {
        Ok(self.chain_data.cancun_time)
    }

    fn get_shanghai_time(&self) -> Result<Option<u64>, StoreError> {
        Ok(self.chain_data.shanghai_time)
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
    fn update_latest_total_difficulty(
        &mut self,
        latest_total_difficulty: U256,
    ) -> Result<(), StoreError> {
        self.chain_data
            .latest_total_difficulty
            .replace(latest_total_difficulty);
        Ok(())
    }

    fn get_latest_block_number(&self) -> Result<Option<BlockNumber>, StoreError> {
        Ok(self.chain_data.latest_block_number)
    }

    fn get_latest_total_difficulty(&self) -> Result<Option<U256>, StoreError> {
        Ok(self.chain_data.latest_total_difficulty)
    }

    fn update_pending_block_number(&mut self, block_number: BlockNumber) -> Result<(), StoreError> {
        self.chain_data.pending_block_number.replace(block_number);
        Ok(())
    }

    fn get_pending_block_number(&self) -> Result<Option<BlockNumber>, StoreError> {
        Ok(self.chain_data.pending_block_number)
    }
}

impl Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("In Memory Store").finish()
    }
}
