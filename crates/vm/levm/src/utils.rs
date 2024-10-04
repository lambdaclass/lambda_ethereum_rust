use crate::{
    block::BlockEnv,
    operations::Operation,
    transaction::{TransactTo, TxEnv},
    vm::{Account, Db, VM},
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
    new_vm_with_ops_addr_bal(bytecode, Address::from_low_u64_be(100), U256::MAX)
}

pub fn new_vm_with_ops(operations: &[Operation]) -> VM {
    let bytecode = ops_to_bytecde(operations);
    new_vm_with_ops_addr_bal(bytecode, Address::from_low_u64_be(100), U256::MAX)
}

pub fn new_vm_with_ops_addr_bal(bytecode: Bytes, address: Address, balance: U256) -> VM {
    let tx_env = TxEnv {
        msg_sender: address,
        chain_id: Some(1),
        transact_to: TransactTo::Call(Address::from_low_u64_be(42)),
        gas_limit: Default::default(),
        gas_price: Default::default(),
        value: Default::default(),
        data: Default::default(),
        nonce: Default::default(),
        access_list: Default::default(),
        max_priority_fee_per_gas: Default::default(),
        blob_hashes: Default::default(),
        max_fee_per_blob_gas: Default::default(),
    };

    let block_env = BlockEnv {
        number: Default::default(),
        coinbase: Default::default(),
        timestamp: Default::default(),
        base_fee_per_gas: Default::default(),
        gas_limit: Default::default(),
        chain_id: Default::default(),
        prev_randao: Default::default(),
        excess_blob_gas: Default::default(),
        blob_gas_used: Default::default(),
    };

    let accounts = [
        (
            Address::from_low_u64_be(42),
            Account {
                balance: U256::MAX,
                bytecode,
                storage: HashMap::new(),
                nonce: 0,
            },
        ),
        (
            address,
            Account {
                balance,
                bytecode: Bytes::default(),
                storage: HashMap::new(),
                nonce: 0,
            },
        ),
    ];

    let state = Db {
        accounts: accounts.into(),
        block_hashes: Default::default(),
    };

    // add the account with code to call

    // add the account passed by parameter

    VM::new(tx_env, block_env, state)
}
