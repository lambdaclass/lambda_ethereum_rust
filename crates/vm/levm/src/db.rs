use crate::vm::{Account, AccountInfo, StorageSlot};
use ethereum_types::{Address, U256};
use keccak_hash::H256;
use std::collections::HashMap;

pub trait Database: std::fmt::Debug {
    fn get_account_info(&self, address: Address) -> AccountInfo;
    fn get_storage_slot(&self, address: Address, key: U256) -> U256;
}

#[derive(Debug, Default)]
pub struct Db {
    accounts: HashMap<Address, Account>,
    block_hashes: HashMap<U256, H256>,
}

// Methods here are for testing purposes only, for initializing the Db with some values
impl Db {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            block_hashes: HashMap::new(),
        }
    }

    /// Builder method with accounts [for testing only]
    pub fn with_accounts(mut self, accounts: HashMap<Address, Account>) -> Self {
        self.accounts = accounts;
        self
    }

    /// Builder method with block hashes [for testing only]
    pub fn with_block_hashes(mut self, block_hashes: HashMap<U256, H256>) -> Self {
        self.block_hashes = block_hashes;
        self
    }
}

impl Database for Db {
    fn get_account_info(&self, address: Address) -> AccountInfo {
        self.accounts
            .get(&address)
            .unwrap_or(&Account::default())
            .info
            .clone()
    }

    fn get_storage_slot(&self, address: Address, key: U256) -> U256 {
        // both `original_value` and `current_value` should work here because they have the same values on Db
        self.accounts
            .get(&address)
            .unwrap_or(&Account::default())
            .storage
            .get(&key)
            .unwrap_or(&StorageSlot::default())
            .original_value
    }

    // fn read_account_storage(&self, address: &Address, key: &U256) -> Option<StorageSlot> {
    //     self.accounts
    //         .get(address)
    //         .and_then(|account| account.storage.get(key))
    //         .cloned()
    // }

    // fn write_account_storage(&mut self, address: &Address, key: U256, slot: StorageSlot) {
    //     self.accounts
    //         .entry(*address)
    //         .or_default()
    //         .storage
    //         .insert(key, slot);
    // }

    // fn get_account_bytecode(&self, address: &Address) -> Bytes {
    //     self.accounts
    //         .get(address)
    //         .map_or(Bytes::new(), |acc| acc.bytecode.clone())
    // }

    // fn balance(&mut self, address: &Address) -> U256 {
    //     self.accounts
    //         .get(address).unwrap().balance
    // }

    // /// Returns the account associated with the given address.
    // /// If the account does not exist in the Db, it creates a new one with the given address.
    // fn get_account(&mut self, address: &Address) -> Result<Account, VMError> {
    //     if self.accounts.contains_key(address) {
    //         return Ok(self.accounts.get(address).unwrap().clone());
    //     }

    //     let new_account = Account {
    //         address: *address,
    //         ..Default::default()
    //     };

    //     self.accounts.insert(*address, new_account);

    //     Ok(self.accounts.get(address).unwrap().clone())
    // }

    // fn get_block_hash(&self, block_number: U256) -> Option<H256> {
    //     self.block_hashes.get(&block_number).cloned()
    // }
}

#[derive(Debug, Default, Clone)]
pub struct Cache {
    pub accounts: HashMap<Address, Account>,
}

impl Cache {
    pub fn get_account(&self, address: Address) -> Option<&Account> {
        self.accounts.get(&address)
    }
    pub fn add_account(&mut self, address: &Address, account: &Account) {
        self.accounts.insert(*address, account.clone());
    }

    pub fn increment_account_nonce(&mut self, address: &Address) {
        if let Some(account) = self.accounts.get_mut(address) {
            account.info.nonce += 1;
        }
    }

    pub fn is_account_cached(&self, address: &Address) -> bool {
        self.accounts.get(address).is_some()
    }
    pub fn is_slot_cached(&self, address: &Address, key: U256) -> bool {
        self.is_account_cached(address)
            && self
                .get_account(*address)
                .map(|account| account.storage.contains_key(&key))
                .unwrap_or(false)
    }
}