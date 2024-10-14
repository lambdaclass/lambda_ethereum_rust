use crate::{
    block::BlockEnv,
    env::Env,
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
        caller: address,
        // msg_sender: address,
        chain_id: Some(1),
        transact_to: TransactTo::Call(Address::from_low_u64_be(42)),
        ..Default::default()
    };

    let block_env = BlockEnv {
        ..Default::default()
    };

    let accounts = [
        (
            Address::from_low_u64_be(42),
            Account {
                address: Address::from_low_u64_be(42),
                balance: U256::MAX,
                bytecode,
                storage: HashMap::new(),
                nonce: 0,
            },
        ),
        (
            address,
            Account {
                address,
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

    let env = Env { tx_env, block_env };

    VM::new(env, state)
}
