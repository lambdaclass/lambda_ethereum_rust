mod db;
mod errors;
mod execution_result;

use db::StoreWrapper;
use ethereum_rust_core::types::{BlockHeader, Transaction, TxKind};
use ethereum_rust_storage::Store;
use revm::{
    db::states::bundle_state::BundleRetention, inspector_handle_register, inspectors::TracerEip3155, primitives::{BlockEnv, TxEnv, B256, U256}, Database, Evm
};
use revm_inspectors::access_list::AccessListInspector;
// Rename imported types for clarity
use revm::primitives::Address as RevmAddress;
use revm::primitives::TxKind as RevmTxKind;
// Export needed types
pub use errors::EvmError;
pub use execution_result::*;
pub use revm::primitives::SpecId;

/// State used when running the EVM
// Encapsulates state behaviour to be agnostic to the evm implementation for crate users
pub struct EvmState(revm::db::State<StoreWrapper>);

// Executes a single tx, doesn't perform state transitions
pub fn execute_tx(
    tx: &Transaction,
    header: &BlockHeader,
    state: &mut EvmState,
    spec_id: SpecId,
) -> Result<ExecutionResult, EvmError> {
    let block_env = block_env(header);
    let tx_env = tx_env(tx);
    run_evm(tx_env, block_env, state, spec_id)
}

/// Runs EVM, doesn't perform state transitions, but stores them
fn run_evm(
    tx_env: TxEnv,
    block_env: BlockEnv,
    state: &mut EvmState,
    spec_id: SpecId,
) -> Result<ExecutionResult, EvmError> {
    let tx_result = {
        let mut evm = Evm::builder()
            .with_db(&mut state.0)
            .with_block_env(block_env)
            .with_tx_env(tx_env)
            .with_spec_id(spec_id)
            .reset_handler()
            .with_external_context(
                TracerEip3155::new(Box::new(std::io::stderr())).without_summary(),
            )
            .append_handler_register(inspector_handle_register)
            .build();
        evm.transact_commit().map_err(EvmError::from)?
    };
    Ok(tx_result.into())
}

pub fn create_access_list(
    tx: &Transaction,
    header: &BlockHeader,
    state: &mut EvmState,
    spec_id: SpecId,
) -> Result<ExecutionResult, EvmError> {
    let tx_env = tx_env(tx);
    let block_env = block_env(header);
    let access_list_inspector = AccessListInspector::new(
        AccessList(tx_env.access_list),
        tx_env.caller,
        match tx_env.transact_to {
            RevmTxKind::Create => {
                state.0.basic(tx_env.caller)?.nonce;
                tx_env.caller.create(tx_env.nonce.unwrap_or_default())
            },
            RevmTxKind::Call(address) => address,
        },
        precompiles
    );
        let tx_result = {let mut evm = Evm::builder()
            .with_db(&mut state.0)
            .with_block_env(block_env)
            .with_tx_env(tx_env)
            .with_spec_id(spec_id)
            .reset_handler()
            .with_external_context(
                &access_list_inspector,
            )
            .build();
        evm.transact_commit().map_err(EvmError::from)?
        };
        let access_list = access_list_inspector.into_access_list();
    Ok(tx_result.into())
}

// Merges transitions stored when executing transactions and applies the resulting changes to the DB
pub fn apply_state_transitions(state: &mut EvmState) {
    state.0.merge_transitions(BundleRetention::Reverts);
    let _bundle = state.0.take_bundle();
    // TODO: Apply bundle to DB
    unimplemented!("Apply state transitions to DB")
}

/// Builds EvmState from a Store
pub fn evm_state(store: Store) -> EvmState {
    EvmState(
        revm::db::State::builder()
            .with_database(StoreWrapper(store))
            .with_bundle_update()
            .without_state_clear()
            .build(),
    )
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
