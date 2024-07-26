mod db;
mod errors;
mod execution_result;

use db::StoreWrapper;
use ethereum_rust_core::{
    types::{BlockHeader, GenericTransaction, Transaction, TxKind},
    Address, H256,
};
use ethereum_rust_storage::Store;
use revm::inspector_handle_register;
use revm::{
    db::states::bundle_state::BundleRetention,
    inspector_handle_register,
    inspectors::TracerEip3155,
    precompile::{PrecompileSpecId, Precompiles},
    primitives::{BlockEnv, TxEnv, B256, U256},
    Database, Evm,
};
use revm_inspectors::access_list::AccessListInspector;
// Rename imported types for clarity
use revm::primitives::{Address as RevmAddress, TxKind as RevmTxKind};
use revm_primitives::{AccessList as RevmAccessList, AccessListItem as RevmAccessListItem};
// Export needed types
pub use errors::EvmError;
pub use execution_result::*;
pub use revm::primitives::SpecId;

type AccessList = Vec<(Address, Vec<H256>)>;

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
            .build();
        evm.transact_commit().map_err(EvmError::from)?
    };
    Ok(tx_result.into())
}

/// Runs the transaction and returns the access list for it
pub fn create_access_list(
    tx: &GenericTransaction,
    header: &BlockHeader,
    state: &mut EvmState,
    spec_id: SpecId,
) -> Result<(ExecutionResult, AccessList), EvmError> {
    let mut tx_env = tx_env_from_generic(tx);
    let block_env = block_env(header);
    // Run tx with access list inspector
    let (execution_result, access_list) = dbg!(create_access_list_inner(
        tx_env.clone(),
        block_env.clone(),
        state,
        spec_id
    ))?;
    // Run the tx with the resulting access list and estimate its fee
    let execution_result = if execution_result.is_success() {
        tx_env.access_list.extend(access_list.0.iter().map(|item| {
            (
                item.address,
                item.storage_keys
                    .iter()
                    .map(|b| U256::from_be_slice(b.as_slice()))
                    .collect(),
            )
        }));
        dbg!(estimate_gas(tx_env, block_env, state, spec_id))?
    } else {
        dbg!(execution_result)
    };
    let access_list: Vec<(Address, Vec<H256>)> = access_list
        .iter()
        .map(|item| {
            (
                Address::from_slice(item.address.0.as_slice()),
                item.storage_keys
                    .iter()
                    .map(|v| H256::from_slice(v.as_slice()))
                    .collect(),
            )
        })
        .collect();
    Ok((execution_result, access_list))
}

/// Runs the transaction and returns the access list for it
fn create_access_list_inner(
    tx_env: TxEnv,
    block_env: BlockEnv,
    state: &mut EvmState,
    spec_id: SpecId,
) -> Result<(ExecutionResult, RevmAccessList), EvmError> {
    let mut access_list_inspector = access_list_inspector(&tx_env, state, spec_id)?;
    let tx_result = {
        let mut evm = Evm::builder()
            .with_db(&mut state.0)
            .with_block_env(block_env)
            .with_tx_env(tx_env)
            .with_spec_id(spec_id)
            .modify_cfg_env(|env| {
                env.disable_base_fee = true;
                env.disable_block_gas_limit = true
            })
            .with_external_context(&mut access_list_inspector)
            .append_handler_register(inspector_handle_register)
            .build();
        evm.transact().map_err(EvmError::from)?
    };

    let access_list = access_list_inspector.into_access_list();
    Ok((tx_result.result.into(), access_list))
}

/// Runs the transaction and returns the estimated gas
fn estimate_gas(
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
            .modify_cfg_env(|env| {
                env.disable_base_fee = true;
                env.disable_block_gas_limit = true
            })
            .build();
        evm.transact().map_err(EvmError::from)?
    };
    Ok(tx_result.result.into())
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
    let mut max_fee_per_blob_gas_bytes: [u8; 32] = [0; 32];
    let max_fee_per_blob_gas = match tx.max_fee_per_blob_gas() {
        Some(x) => {
            x.to_big_endian(&mut max_fee_per_blob_gas_bytes);
            Some(U256::from_be_bytes(max_fee_per_blob_gas_bytes))
        }
        None => None,
    };
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
        max_fee_per_blob_gas,
    }
}

// Used to estimate gas and create access lists
fn tx_env_from_generic(tx: &GenericTransaction) -> TxEnv {
    TxEnv {
        caller: RevmAddress(tx.from.0.into()),
        gas_limit: tx.gas.unwrap_or(0x23f3e20), // Ensure tx doesn't fail due to gas limit
        gas_price: U256::from(tx.gas_price),
        transact_to: match tx.to {
            TxKind::Call(address) => RevmTxKind::Call(address.0.into()),
            TxKind::Create => RevmTxKind::Create,
        },
        value: U256::from_limbs(tx.value.0),
        data: tx.input.clone().into(),
        nonce: Some(tx.nonce),
        chain_id: tx.chain_id,
        access_list: tx
            .access_list
            .iter()
            .map(|entry| {
                (
                    RevmAddress(entry.address.0.into()),
                    entry
                        .storage_keys
                        .iter()
                        .map(|a| U256::from_be_bytes(a.0))
                        .collect(),
                )
            })
            .collect(),
        gas_priority_fee: tx.max_priority_fee_per_gas.map(U256::from),
        blob_hashes: tx
            .blob_versioned_hashes
            .iter()
            .map(|hash| B256::from(hash.0))
            .collect(),
        max_fee_per_blob_gas: tx.max_fee_per_blob_gas.map(U256::from),
    }
}

// Creates an AccessListInspector that will collect the accesses used by the evm execution
fn access_list_inspector(
    tx_env: &TxEnv,
    state: &mut EvmState,
    spec_id: SpecId,
) -> Result<AccessListInspector, EvmError> {
    // Access list provided by the transaction
    let current_access_list = RevmAccessList(
        tx_env
            .access_list
            .iter()
            .map(|(addr, list)| RevmAccessListItem {
                address: *addr,
                storage_keys: list.iter().map(|v| B256::from(v.to_be_bytes())).collect(),
            })
            .collect(),
    );
    // Addresses accessed when using precompiles
    let precompile_addresses = Precompiles::new(PrecompileSpecId::from_spec_id(spec_id))
        .addresses()
        .cloned();
    // Address that is either called or created by the transaction
    let to = match tx_env.transact_to {
        RevmTxKind::Call(address) => address,
        RevmTxKind::Create => {
            let nonce = state
                .0
                .basic(tx_env.caller)?
                .map(|info| info.nonce)
                .unwrap_or_default();
            tx_env.caller.create(nonce)
        }
    };
    Ok(AccessListInspector::new(
        current_access_list,
        tx_env.caller,
        to,
        precompile_addresses,
    ))
}
