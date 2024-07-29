mod db;
mod errors;
mod execution_result;

use db::StoreWrapper;
use ethereum_rust_core::{
    types::{AccountInfo, BlockHeader, Transaction, TxKind},
    Address, BigEndianHash, H256, U256,
};
use ethereum_rust_storage::{error::StoreError, Store};
use revm::{
    db::states::bundle_state::BundleRetention,
    inspectors::TracerEip3155,
    primitives::{BlockEnv, TxEnv, B256, U256 as RevmU256},
    Evm,
};
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

impl EvmState {
    /// Get a reference to inner `Store` database
    pub fn database(&self) -> &Store {
        &self.0.database.0
    }
}

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

// Merges transitions stored when executing transactions and applies the resulting changes to the DB
pub fn apply_state_transitions(state: &mut EvmState) -> Result<(), StoreError> {
    state.0.merge_transitions(BundleRetention::PlainState);
    let bundle = state.0.take_bundle();
    // Update accounts
    for (address, account) in bundle.state() {
        if account.status.is_not_modified() {
            continue;
        }
        let address = Address::from_slice(address.0.as_slice());
        // Remove account from DB if destroyed
        if account.status.was_destroyed() {
            state.database().remove_account(address)?;
        }
        // Apply account changes to DB
        // If the account was changed then both original and current info will be present
        if account.is_info_changed() {
            // Update account info in DB
            if let Some(new_acc_info) = account.account_info() {
                let code_hash = H256::from_slice(new_acc_info.code_hash.as_slice());
                let account_info = AccountInfo {
                    code_hash,
                    balance: U256::from_little_endian(new_acc_info.balance.as_le_slice()),
                    nonce: new_acc_info.nonce,
                };
                state.database().add_account_info(address, account_info)?;

                if account.is_contract_changed() {
                    // Update code in db
                    if let Some(code) = new_acc_info.code {
                        state
                            .database()
                            .add_account_code(code_hash, code.original_bytes().clone().0)?;
                    }
                }
            }
        }
        // Update account storage in DB
        for (key, slot) in account.storage.iter() {
            if slot.is_changed() {
                state.database().add_storage_at(
                    address,
                    H256::from_uint(&U256::from_little_endian(key.as_le_slice())),
                    H256::from_uint(&U256::from_little_endian(
                        slot.present_value().as_le_slice(),
                    )),
                )?;
            }
        }
    }
    Ok(())
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
        number: RevmU256::from(header.number),
        coinbase: RevmAddress(header.coinbase.0.into()),
        timestamp: RevmU256::from(header.timestamp),
        gas_limit: RevmU256::from(header.gas_limit),
        basefee: RevmU256::from(header.base_fee_per_gas),
        difficulty: RevmU256::from_limbs(header.difficulty.0),
        prevrandao: Some(header.prev_randao.as_fixed_bytes().into()),
        ..Default::default()
    }
}

fn tx_env(tx: &Transaction) -> TxEnv {
    let mut max_fee_per_blob_gas_bytes: [u8; 32] = [0; 32];
    let max_fee_per_blob_gas = match tx.max_fee_per_blob_gas() {
        Some(x) => {
            x.to_big_endian(&mut max_fee_per_blob_gas_bytes);
            Some(RevmU256::from_be_bytes(max_fee_per_blob_gas_bytes))
        }
        None => None,
    };
    TxEnv {
        caller: RevmAddress(tx.sender().0.into()),
        gas_limit: tx.gas_limit(),
        gas_price: RevmU256::from(tx.gas_price()),
        transact_to: match tx.to() {
            TxKind::Call(address) => RevmTxKind::Call(address.0.into()),
            TxKind::Create => RevmTxKind::Create,
        },
        value: RevmU256::from_limbs(tx.value().0),
        data: tx.data().clone().into(),
        nonce: Some(tx.nonce()),
        chain_id: tx.chain_id(),
        access_list: tx
            .access_list()
            .into_iter()
            .map(|(addr, list)| {
                (
                    RevmAddress(addr.0.into()),
                    list.into_iter()
                        .map(|a| RevmU256::from_be_bytes(a.0))
                        .collect(),
                )
            })
            .collect(),
        gas_priority_fee: tx.max_priority_fee().map(RevmU256::from),
        blob_hashes: tx
            .blob_versioned_hashes()
            .into_iter()
            .map(|hash| B256::from(hash.0))
            .collect(),
        max_fee_per_blob_gas,
    }
}
