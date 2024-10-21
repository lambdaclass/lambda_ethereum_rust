use crate::{
    call_frame::Log,
    primitives::{Address, Bytes},
    vm::Account,
};
use std::{collections::HashMap, vec::Vec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxResult {
    Success,
    Revert,
    ExceptionalHalt(VmError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    UnknownError,
    OutOfGas,
    MemoryOverflow,
    ContractSizeLimit,
    NonceOverflow,
    ContractAlreadyExists,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionReport {
    pub result: TxResult,
    pub new_state: HashMap<Address, Account>,
    pub gas_used: u64,
    pub gas_refunded: u64,
    pub output: Bytes,
    pub logs: Vec<Log>,
    // This only applies to create transactions. It's fundamentally ambiguous since
    // a transaction could create multiple new contracts, but whatever.
    pub created_address: Option<Address>,
}

impl TransactionReport {
    pub fn is_success(&self) -> bool {
        matches!(self.result, TxResult::Success)
    }
}
