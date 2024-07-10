use super::{Key, StoreEngine, Value};
use anyhow::Result;
use ethereum_rust_core::types::AccountInfo;
use ethereum_types::Address;
use std::{collections::HashMap, fmt::Debug};

#[derive(Default)]
pub struct Store {
    account_infos: HashMap<Address, AccountInfo>,
    values: HashMap<Key, Value>,
}

impl Store {
    pub fn new() -> Result<Self> {
        Ok(Self {
            account_infos: HashMap::new(),
            values: HashMap::new(),
        })
    }
}

impl StoreEngine for Store {
    fn add_account_info(&mut self, address: Address, account_info: AccountInfo) -> Result<()> {
        self.account_infos.insert(address, account_info);
        Ok(())
    }

    fn get_account_info(&self, address: Address) -> Result<Option<AccountInfo>> {
        Ok(self.account_infos.get(&address).cloned())
    }

    fn set_value(&mut self, key: Key, value: Value) -> Result<()> {
        let _ = self.values.insert(key, value);
        Ok(())
    }

    fn get_value(&self, key: Key) -> Result<Option<Vec<u8>>, anyhow::Error> {
        Ok(self.values.get(&key).cloned())
    }
}

impl Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("In Memory Store").finish()
    }
}
