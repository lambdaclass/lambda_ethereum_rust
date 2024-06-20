use std::{collections::HashMap, io::stderr};

use revm::{
    inspector_handle_register,
    inspectors::TracerEip3155,
    primitives::{
        keccak256, AccountInfo, Address, Bytecode, Env, ExecutionResult, SpecId, TransactTo, U256,
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

    env.block.number = block.number;
    env.block.coinbase = block.coinbase;
    env.block.timestamp = block.timestamp;
    env.block.gas_limit = block.gas_limit;
    env.block.basefee = U256::ZERO;
    env.block.difficulty = U256::ZERO;
    env.block.prevrandao = Some(block.mix_hash);

    env.tx.caller = transaction.sender;

    env.tx.gas_price = transaction
        .gas_price
        .or(transaction.max_fee_per_gas)
        .unwrap_or_default();
    env.tx.gas_priority_fee = transaction.max_priority_fee_per_gas;

    let spec_id = SpecId::CANCUN;

    env.tx.gas_limit = transaction.gas_limit.saturating_to();

    env.tx.data = transaction.data.clone();
    env.tx.value = transaction.value;

    env.tx.transact_to = TransactTo::Call(transaction.to);

    let mut cache_state = revm::CacheState::new(false);
    for (address, info) in pre {
        let acc_info = AccountInfo {
            balance: info.balance,
            code_hash: keccak256(&info.code),
            code: Some(Bytecode::new_raw(info.code)),
            nonce: info.nonce.saturating_to(), // TODO ver
        };
        cache_state.insert_account_with_storage(address, acc_info, info.storage);
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
