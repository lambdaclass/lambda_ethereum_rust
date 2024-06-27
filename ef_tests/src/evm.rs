use std::{collections::HashMap, io::stderr};

use ethrex_core::{Address, U256};
use revm::{
    inspector_handle_register,
    inspectors::TracerEip3155,
    primitives::{
        keccak256, AccountInfo, Bytecode, Env, ExecutionResult, FixedBytes, SpecId, TransactTo,
        U256 as AlloyU256,
    },
    Evm,
};

use crate::types::{Account, Header, Transaction};

pub fn execute_transaction(
    block: &Header,
    transaction: &Transaction,
    pre: HashMap<Address, Account>,
) -> ExecutionResult {
    let mut env = Box::<Env>::default();

    env.block.number = to_alloy_bytes(block.number);
    env.block.coinbase = block.coinbase.to_fixed_bytes().into();
    env.block.timestamp = to_alloy_bytes(block.timestamp);
    env.block.gas_limit = to_alloy_bytes(block.gas_limit);
    env.block.basefee = AlloyU256::ZERO;
    env.block.difficulty = AlloyU256::ZERO;
    env.block.prevrandao = Some(block.mix_hash.as_fixed_bytes().into());

    env.tx.caller = transaction.sender.to_fixed_bytes().into();

    env.tx.gas_price = to_alloy_bytes(
        transaction
            .gas_price
            .or(transaction.max_fee_per_gas)
            .unwrap_or_default(),
    );
    env.tx.gas_priority_fee = transaction.max_priority_fee_per_gas.map(to_alloy_bytes);

    let spec_id = SpecId::CANCUN;

    env.tx.gas_limit = transaction.gas_limit.as_u64();

    env.tx.data = transaction.data.clone();
    env.tx.value = to_alloy_bytes(transaction.value);

    env.tx.transact_to = TransactTo::Call(transaction.to.to_fixed_bytes().into());

    let mut cache_state = revm::CacheState::new(false);
    for (address, info) in pre {
        let acc_info = AccountInfo {
            balance: to_alloy_bytes(info.balance),
            code_hash: keccak256(&info.code),
            code: Some(Bytecode::new_raw(info.code)),
            nonce: info.nonce.as_u64(),
        };

        let mut storage = HashMap::new();
        for (k, v) in info.storage {
            storage.insert(to_alloy_bytes(k), to_alloy_bytes(v));
        }

        cache_state.insert_account_with_storage(address.to_fixed_bytes().into(), acc_info, storage);
    }

    let cache = cache_state.clone();
    let mut state = revm::db::State::builder()
        .with_cached_prestate(cache)
        .with_bundle_update()
        .build();
    let evm = Evm::builder()
        .with_db(&mut state)
        .modify_env(|e| e.clone_from(&env))
        .with_spec_id(spec_id)
        .build();

    let mut evm = evm
        .modify()
        .reset_handler_with_external_context(
            TracerEip3155::new(Box::new(stderr())).without_summary(),
        )
        .append_handler_register(inspector_handle_register)
        .build();

    evm.transact_commit().unwrap()
}

fn to_alloy_bytes(eth_byte: U256) -> AlloyU256 {
    let mut bytes = [0u8; 32];
    eth_byte.to_big_endian(&mut bytes);
    let fixed_bytes: FixedBytes<32> = bytes.into();
    fixed_bytes.into()
}
