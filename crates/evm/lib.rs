mod database;
mod errors;
mod execution_result;

use database::StoreWrapper;
use ethereum_rust_core::{
    types::{Account, BlockHeader, Transaction, TxKind},
    Address,
};
use ethereum_rust_storage::{EngineType, Store};
use revm::{
    inspector_handle_register, inspectors::TracerEip3155, primitives::{BlockEnv, Bytecode, TxEnv, B256, U256}, CacheState, DatabaseCommit, Evm
};
use std::collections::HashMap;
// Rename imported types for clarity
use revm::primitives::Address as RevmAddress;
use revm::primitives::TxKind as RevmTxKind;
// Export needed types
pub use errors::EvmError;
pub use execution_result::*;
pub use revm::primitives::SpecId;

// Executes a single tx, doesn't perform state transitions
pub fn execute_tx(
    tx: &Transaction,
    header: &BlockHeader,
    _pre: &HashMap<Address, Account>, // TODO: Modify this type when we have a defined State structure
    spec_id: SpecId,
) -> Result<ExecutionResult, EvmError> {
    let state = StoreWrapper(Store::new("temp.db", EngineType::InMemory).unwrap());
    let block_env = block_env(header);
    let tx_env = tx_env(tx);
    let mut state = revm::db::State::builder()
        .with_database(state)
        .with_bundle_update()
        .without_state_clear()
        .build();
    let tx_result = {let mut evm = Evm::builder()
        .with_db(&mut state)
        .with_block_env(block_env)
        .with_tx_env(tx_env)
        .with_spec_id(spec_id)
        .reset_handler()
        .with_external_context(TracerEip3155::new(Box::new(std::io::stderr())).without_summary())
        .append_handler_register(inspector_handle_register)
        .build();
        evm.transact_commit().map_err(EvmError::from)?
    };
    state.merge_transitions(revm::db::states::bundle_state::BundleRetention::Reverts);
    let bundle = state.bundle_state;

    Ok(tx_result.into())
}

/// Runs EVM, doesn't perform state transitions
pub fn run_evm(
    tx_env: TxEnv,
    block_env: BlockEnv,
    db: &mut revm::db::State<StoreWrapper>,
    spec_id: SpecId,
) -> Result<ExecutionResult, EvmError> {
    let tx_result = {let mut evm = Evm::builder()
        .with_db(db)
        .with_block_env(block_env)
        .with_tx_env(tx_env)
        .with_spec_id(spec_id)
        .reset_handler()
        .with_external_context(TracerEip3155::new(Box::new(std::io::stderr())).without_summary())
        .append_handler_register(inspector_handle_register)
        .build();
        evm.transact_commit().map_err(EvmError::from)?
    };
    Ok(tx_result.into())
}

fn block_env(header: &BlockHeader) -> BlockEnv {
    BlockEnv {
        number: U256::from(header.number),
        coinbase: RevmAddress(header.coinbase.0.into()),
        timestamp: U256::from(header.timestamp),
        gas_limit: U256::from(header.gas_limit),
        basefee: U256::from(header.base_fee_per_gas),
        difficulty: U256::from_limbs(header.difficulty.0),
        prevrandao: Some(header.prev_randao.as_fixed_bytes().into()),
        ..Default::default()
    }
}

fn tx_env(tx: &Transaction) -> TxEnv {
    TxEnv {
        caller: RevmAddress(tx.sender().0.into()),
        gas_limit: tx.gas_limit(),
        gas_price: U256::from(tx.gas_price()),
        transact_to: match tx.to() {
            TxKind::Call(address) => RevmTxKind::Call(address.0.into()),
            TxKind::Create => RevmTxKind::Create,
        },
        value: U256::from_limbs(tx.value().0),
        data: tx.data().clone().into(),
        nonce: Some(tx.nonce()),
        chain_id: tx.chain_id(),
        access_list: tx
            .access_list()
            .into_iter()
            .map(|(addr, list)| {
                (
                    RevmAddress(addr.0.into()),
                    list.into_iter().map(|a| U256::from_be_bytes(a.0)).collect(),
                )
            })
            .collect(),
        gas_priority_fee: tx.max_priority_fee().map(U256::from),
        blob_hashes: tx
            .blob_versioned_hashes()
            .into_iter()
            .map(|hash| B256::from(hash.0))
            .collect(),
        max_fee_per_blob_gas: tx.max_fee_per_blob_gas().map(U256::from),
    }
}
