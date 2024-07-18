use super::{Key, StoreEngine, Value};
use crate::error::StoreError;
use bytes::Bytes;
use ethereum_rust_core::types::{AccountInfo, BlockBody, BlockHash, BlockHeader, BlockNumber};
use ethereum_types::{Address, H256};
use std::{collections::HashMap, fmt::Debug};

#[derive(Default)]
pub struct Store {
    account_infos: HashMap<Address, AccountInfo>,
    block_numbers: HashMap<BlockHash, BlockNumber>,
    bodies: HashMap<BlockNumber, BlockBody>,
    headers: HashMap<BlockNumber, BlockHeader>,
    values: HashMap<Key, Value>,
    // Maps code hashes to code
    account_codes: HashMap<H256, Bytes>,
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

    fn set_value(&mut self, key: Key, value: Value) -> Result<(), StoreError> {
        let _ = self.values.insert(key, value);
        Ok(())
    }

    fn get_value(&self, key: Key) -> Result<Option<Vec<u8>>, StoreError> {
        Ok(self.values.get(&key).cloned())
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

    fn add_account_code(&mut self, code_hash: H256, code: Bytes) -> Result<(), StoreError> {
        self.account_codes.insert(code_hash, code);
        Ok(())
    }

    fn get_account_code(&self, code_hash: H256) -> Result<Option<Bytes>, StoreError> {
        Ok(self.account_codes.get(&code_hash).cloned())
    }
}

impl Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("In Memory Store").finish()
    }
}
