/*
Original work Copyright (c) 2023 [Succinct Labs]
Modified work Copyright 2024  Lambdaclass

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.
*/

// Types are not compatible with DatabaseRef trait
//use ethereum_rust_core::{types::AccountInfo, Address, H256, U256};

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use revm::primitives::{db::DatabaseRef, AccountInfo, Address, Bytecode, B256, KECCAK_EMPTY, U256};

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryDB {
    pub accounts: HashMap<Address, AccountInfo>,
    pub storage: HashMap<Address, HashMap<U256, U256>>,
    pub block_hashes: HashMap<u64, B256>,
}

impl Default for MemoryDB {
    fn default() -> Self {
        let mut accounts = HashMap::new();
        let mut storage = HashMap::new();
        let mut block_hashes = HashMap::new();

        // Insert default accounts
        accounts.insert(Address::default(), AccountInfo::default());

        // Insert default storage
        let mut default_storage = HashMap::new();
        default_storage.insert(U256::from(0), U256::from(0));
        storage.insert(Address::default(), default_storage);

        // Insert a default block hash
        block_hashes.insert(0_u64, KECCAK_EMPTY);

        MemoryDB {
            accounts,
            storage,
            block_hashes,
        }
    }
}

// Constructor to create MemoryDB from given HashMaps
impl MemoryDB {
    pub fn new(
        accounts: HashMap<Address, AccountInfo>,
        storage: HashMap<Address, HashMap<U256, U256>>,
        block_hashes: HashMap<u64, B256>,
    ) -> Self {
        MemoryDB {
            accounts,
            storage,
            block_hashes,
        }
    }
}

// This should be changed
use reth_storage_errors::provider::ProviderError;

// from rsp:
// https://github.com/succinctlabs/rsp/blob/3647076da6580e30384dd911a3fc50d4bcdb5bc1/crates/storage/witness-db/src/lib.rs#L20
impl DatabaseRef for MemoryDB {
    // Should be a custom error
    type Error = ProviderError;

    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        // Even absent accounts are loaded as `None`, so if an entry is missing from `HashMap` we
        // need to panic. Otherwise it would be interpreted by `revm` as an uninitialized account.
        Ok(Some(self.accounts.get(&address).cloned().unwrap()))
    }

    fn code_by_hash_ref(&self, _code_hash: B256) -> Result<Bytecode, Self::Error> {
        unimplemented!()
    }

    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        // Absence of storage trie or slot must be treated as an error here. Otherwise it's possible
        // to trick `revm` into believing a slot is `0` when it's not.
        Ok(*self.storage.get(&address).unwrap().get(&index).unwrap())
    }

    fn block_hash_ref(&self, number: u64) -> Result<B256, Self::Error> {
        Ok(*self.block_hashes.get(&number).unwrap())
    }
}
