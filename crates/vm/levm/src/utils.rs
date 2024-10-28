use crate::{
    db::{Cache, Db},
    operations::Operation,
    vm::{Account, AccountInfo, VM},
};
use bytes::Bytes;
use ethereum_types::{Address, U256};
use std::collections::HashMap;

pub fn ops_to_bytecde(operations: &[Operation]) -> Bytes {
    operations
        .iter()
        .flat_map(Operation::to_bytecode)
        .collect::<Bytes>()
}

pub fn new_vm_with_bytecode(bytecode: Bytes) -> VM {
    new_vm_with_ops_addr_bal_db(bytecode, Address::from_low_u64_be(100), U256::MAX, Db::new(), Cache::default())
}

pub fn new_vm_with_ops(operations: &[Operation]) -> VM {
    let bytecode = ops_to_bytecde(operations);
    new_vm_with_ops_addr_bal_db(bytecode, Address::from_low_u64_be(100), U256::MAX, Db::new(), Cache::default())
}

pub fn new_vm_with_ops_db(operations: &[Operation], db: Db) -> VM {
    let bytecode = ops_to_bytecde(operations);
    new_vm_with_ops_addr_bal_db(bytecode, Address::from_low_u64_be(100), U256::MAX, db, Cache::default())
}

/// This function is for testing purposes only.
pub fn new_vm_with_ops_addr_bal_db(bytecode: Bytes, address: Address, balance: U256, mut db: Db, mut cache: Cache) -> VM {
    let accounts = [
        (
            Address::from_low_u64_be(42),
            Account {
                info: AccountInfo {
                    nonce: 0,
                    balance: U256::MAX,
                    bytecode
                },
                storage: HashMap::new(),
            },
        ),
        (
            address,
            Account {
                info: AccountInfo {
                    nonce: 0,
                    balance,
                    bytecode: Bytes::default(),
                },                
                storage: HashMap::new(),
            },
        ),
    ];

    db.add_accounts(accounts.to_vec());
    
    // add to cache accounts from list accounts
    cache.add_account(&accounts[0].0, &accounts[0].1);
    cache.add_account(&accounts[1].0, &accounts[1].1);

    // add the account with code to call

    // add the account passed by parameter

    VM::new(
        Address::from_low_u64_be(42),
        address,
        Default::default(),
        Default::default(),
        U256::MAX, // arbitrary gas limit for now...
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
        U256::one(),
        Default::default(),
        Default::default(),
        Box::new(db),
        cache,
        Default::default(),
        Default::default(),
        Default::default(),
    )
}
