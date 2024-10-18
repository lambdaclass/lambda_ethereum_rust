use crate::{
    primitives::{Address, Bytes, U256},
    state::Account,
    syscall::Log,
};
use core::fmt;
use std::{boxed::Box, collections::HashMap, vec::Vec};

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

/// Transaction validation error.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InvalidTransaction {
    TargetContractDoesNotExist,
    CallGasCostMoreThanGasLimit,
    CreateInitCodeSizeLimit,
    BlobGasPriceGreaterThanMax,
    EmptyBlobs,
    BlobCreateTransaction,
    TooManyBlobs { max: usize, have: usize },
    BlobVersionNotSupported,
}

impl fmt::Display for InvalidTransaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CallGasCostMoreThanGasLimit => {
                write!(f, "call gas cost exceeds the gas limit")
            }
            Self::CreateInitCodeSizeLimit => {
                write!(f, "create initcode size limit")
            }
            Self::BlobGasPriceGreaterThanMax => {
                write!(f, "blob gas price is greater than max fee per blob gas")
            }
            Self::EmptyBlobs => write!(f, "empty blobs"),
            Self::BlobCreateTransaction => write!(f, "blob create transaction"),
            Self::TooManyBlobs { max, have } => {
                write!(f, "too many blobs, have {have}, max {max}")
            }
            Self::BlobVersionNotSupported => write!(f, "blob version not supported"),
            Self::TargetContractDoesNotExist => write!(f, "Target Contract does not Exist"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum PrecompileError {
    InvalidCalldata,
    NotEnoughGas,
    Secp256k1Error,
    InvalidEcPoint,
}
