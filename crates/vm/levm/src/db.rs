use std::collections::HashMap;
use bytes::Bytes;
use ethereum_types::{Address, U256};
use keccak_hash::H256;

use crate::{vm::Account, errors::VMError, vm::StorageSlot};

pub trait Database: std::fmt::Debug {
    fn read_account_storage(&self, address: &Address, key: &U256) -> Option<StorageSlot>;
    fn write_account_storage(&mut self, address: &Address, key: U256, slot: StorageSlot);
    fn get_account_bytecode(&self, address: &Address) -> Bytes;
    fn balance(&mut self, address: &Address) -> U256;
    // fn add_account(&mut self, address: Address, account: Account);
    // fn increment_account_nonce(&mut self, address: &Address);
    fn get_account(&mut self, address: &Address) -> Result<Account, VMError>; // Changed from &Account to Account
    // fn insert_account(&mut self, address: Address, account: Account);
    fn get_block_hash(&self, block_number: U256) -> Option<H256>;
    // fn insert_block_hash(&mut self, block_number: U256, block_hash: u64);
}

#[derive(Debug, Default)]
pub struct Db {
    pub accounts: HashMap<Address, Account>,
    // contracts: HashMap<B256, Bytecode>,
    pub block_hashes: HashMap<U256, H256>,
}

impl Db {
    // Methods here are for testing purposes only, real methods are in trait Database
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            block_hashes: HashMap::new(),
        }
    }

    pub fn add_account(&mut self, address: Address, account: Account) {
        self.accounts.insert(address, account);
    }

    pub fn add_block_hash(&mut self, block_number: U256, block_hash: H256) {
        self.block_hashes.insert(block_number, block_hash);
    }
}    

impl Database for Db{

    fn read_account_storage(&self, address: &Address, key: &U256) -> Option<StorageSlot> {
        self.accounts
            .get(address)
            .and_then(|account| account.storage.get(key))
            .cloned()
    }

    fn write_account_storage(&mut self, address: &Address, key: U256, slot: StorageSlot) {
        self.accounts
            .entry(*address)
            .or_default()
            .storage
            .insert(key, slot);
    }

    fn get_account_bytecode(&self, address: &Address) -> Bytes {
        self.accounts
            .get(address)
            .map_or(Bytes::new(), |acc| acc.bytecode.clone())
    }

    fn balance(&mut self, address: &Address) -> U256 {
        self.accounts
            .get(address).unwrap().balance
    }

    /// Returns the account associated with the given address.
    /// If the account does not exist in the Db, it creates a new one with the given address.
    fn get_account(&mut self, address: &Address) -> Result<Account, VMError> {
        if self.accounts.contains_key(address) {
            return Ok(self.accounts.get(address).unwrap().clone());
        }

        let new_account = Account {
            address: *address,
            ..Default::default()
        };

        self.accounts.insert(*address, new_account);

        Ok(self.accounts.get(address).unwrap().clone())
    }

    fn get_block_hash(&self, block_number: U256) -> Option<H256> {
        self.block_hashes.get(&block_number).cloned()
    }
}


#[derive(Debug, Default, Clone)]
pub struct Cache {
    pub accounts: HashMap<Address, Account>,
}

impl Cache {
    pub fn get_account(&self, address: &Address) -> Option<&Account> {
        self.accounts.get(address)
    }
    pub fn add_account(&mut self, account: &Account) {
        self.accounts.insert(account.address, account.clone());
    }

    pub fn increment_account_nonce(&mut self, address: &Address) {
        if let Some(account) = self.accounts.get_mut(address) {
            account.nonce += 1;
        }
    }
}
