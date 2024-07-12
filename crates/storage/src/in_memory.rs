use super::{Key, StoreEngine, Value};
use crate::error::StoreError;
use ethereum_rust_core::types::{AccountInfo, BlockBody, BlockHeader, BlockNumber};
use ethereum_types::Address;
use std::{collections::HashMap, fmt::Debug};

#[derive(Default)]
pub struct Store {
    account_infos: HashMap<Address, AccountInfo>,
    bodies: HashMap<BlockNumber, BlockBody>,
    headers: HashMap<BlockNumber, BlockHeader>,
    values: HashMap<Key, Value>,
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
}

impl Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("In Memory Store").finish()
    }
}
