use super::{
    types::{Account, BlockHeader, Transaction},
    Address,
};
use bytes::Bytes;
use revm::{
    inspector_handle_register,
    inspectors::TracerEip3155,
    primitives::{BlockEnv, Bytecode, TxEnv, TxKind, U256},
    CacheState, Evm,
};
use std::collections::HashMap;
// Rename imported types for clarity
use revm::primitives::result::Output as RevmOutput;
use revm::primitives::result::SuccessReason as RevmSuccessReason;
use revm::primitives::AccountInfo as RevmAccountInfo;
use revm::primitives::Address as RevmAddress;
use revm::primitives::ExecutionResult as RevmExecutionResult;
// Export needed types
pub use revm::primitives::SpecId;

pub enum ExecutionResult {
    Success {
        reason: SuccessReason,
        gas_used: u64,
        gas_refunded: u64,
        output: Output,
    },
    /// Reverted by `REVERT` opcode
    Revert { gas_used: u64, output: Bytes },
    /// Reverted for other reasons, spends all gas.
    Halt {
        reason: String,
        /// Halting will spend all the gas, which will be equal to gas_limit.
        gas_used: u64,
    },
}

pub enum SuccessReason {
    Stop,
    Return,
    SelfDestruct,
    EofReturnContract,
}

pub enum Output {
    Call(Bytes),
    Create(Bytes, Option<Address>),
}

impl Into<ExecutionResult> for RevmExecutionResult {
    fn into(self) -> ExecutionResult {
        match self {
            RevmExecutionResult::Success {
                reason,
                gas_used,
                gas_refunded,
                logs: _,
                output,
            } => ExecutionResult::Success {
                reason: match reason {
                    RevmSuccessReason::Stop => SuccessReason::Stop,
                    RevmSuccessReason::Return => SuccessReason::Return,
                    RevmSuccessReason::SelfDestruct => SuccessReason::SelfDestruct,
                    RevmSuccessReason::EofReturnContract => SuccessReason::EofReturnContract,
                },
                gas_used,
                gas_refunded,
                output: match output {
                    RevmOutput::Call(bytes) => Output::Call(bytes.0),
                    RevmOutput::Create(bytes, addr) => Output::Create(
                        bytes.0,
                        addr.map(|addr| Address::from_slice(addr.0.as_ref())),
                    ),
                },
            },
            RevmExecutionResult::Revert { gas_used, output } => ExecutionResult::Revert {
                gas_used,
                output: output.0,
            },
            RevmExecutionResult::Halt { reason, gas_used } => ExecutionResult::Halt {
                reason: format!("{:?}", reason),
                gas_used,
            },
        }
    }
}

pub fn execute_tx(
    tx: &Transaction,
    header: &BlockHeader,
    pre: &HashMap<Address, Account>, // TODO: Modify this type when we have a defined State structure
    spec_id: SpecId,
) -> ExecutionResult {
    let block_env = block_env(header);
    let tx_env = tx_env(tx);
    let cache_state = cache_state(pre);
    let mut state = revm::db::State::builder()
        .with_cached_prestate(cache_state)
        .with_bundle_update()
        .build();
    let mut evm = Evm::builder()
        .with_db(&mut state)
        .with_block_env(block_env)
        .with_tx_env(tx_env)
        .with_spec_id(spec_id)
        .reset_handler()
        .with_external_context(TracerEip3155::new(Box::new(std::io::stderr())).without_summary())
        .append_handler_register(inspector_handle_register)
        .build();
    let tx_result = evm.transact().unwrap();
    tx_result.result.into()
}

fn cache_state(pre: &HashMap<Address, Account>) -> CacheState {
    let mut cache_state = revm::CacheState::new(false);
    for (address, account) in pre {
        let acc_info = RevmAccountInfo {
            balance: U256::from_limbs(account.info.balance.0),
            code_hash: account.info.code_hash.0.into(),
            code: Some(Bytecode::new_raw(account.code.clone().into())),
            nonce: account.info.nonce,
        };

        let mut storage = HashMap::new();
        for (k, v) in &account.storage {
            storage.insert(U256::from_be_bytes(k.0), U256::from_be_bytes(v.0));
        }

        cache_state.insert_account_with_storage(address.to_fixed_bytes().into(), acc_info, storage);
    }
    cache_state
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
        transact_to: TxKind::Call(RevmAddress(tx.to().0.into())), // Todo: handle case where this is Create
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
        ..Default::default()
    }
}
