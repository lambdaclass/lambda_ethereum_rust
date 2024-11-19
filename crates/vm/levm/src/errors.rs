use crate::account::Account;
use bytes::Bytes;
use ethereum_rust_core::{types::Log, Address};
use std::collections::HashMap;

/// Errors that halt the program
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VMError {
    #[error("Stack underflow")]
    StackUnderflow,
    #[error("Stack overflow")]
    StackOverflow,
    #[error("Invalid jump")]
    InvalidJump,
    #[error("Opcode not allowed in static context")]
    OpcodeNotAllowedInStaticContext,
    #[error("Opcode not found")]
    OpcodeNotFound,
    #[error("Invalid bytecode")]
    InvalidBytecode,
    #[error("Out of gas")]
    OutOfGas,
    #[error("Very large number")]
    VeryLargeNumber,
    #[error("Overflow in arithmetic operation")]
    OverflowInArithmeticOp,
    #[error("Fatal error")]
    FatalError,
    #[error("Invalid transaction")]
    InvalidTransaction,
    #[error("Revert opcode")]
    RevertOpcode,
    #[error("Invalid opcode")]
    InvalidOpcode,
    #[error("Missing blob hashes")]
    MissingBlobHashes,
    #[error("Blob hash index out of bounds")]
    BlobHashIndexOutOfBounds,
    #[error("Sender account does not exist")]
    SenderAccountDoesNotExist,
    #[error("Address does not match an account")]
    AddressDoesNotMatchAnAccount,
    #[error("Sender account should not have bytecode")]
    SenderNotEOA,
    #[error("Sender balance should contain transfer value")]
    InsufficientAccountFunds,
    #[error("Gas price is lower than base fee")]
    GasPriceIsLowerThanBaseFee,
    #[error("Address already occupied")]
    AddressAlreadyOccupied,
    #[error("Contract output too big")]
    ContractOutputTooBig,
    #[error("Invalid initial byte")]
    InvalidInitialByte,
    #[error("Nonce is max (overflow)")]
    NonceIsMax,
    #[error("Memory load out of bounds")]
    MemoryLoadOutOfBounds,
    #[error("Memory store out of bounds")]
    MemoryStoreOutOfBounds,
    #[error("Gas limit price product overflow")]
    GasLimitPriceProductOverflow,
    #[error("Internal error: {0}")]
    Internal(#[from] InternalError),
    #[error("Data size overflow")]
    DataSizeOverflow,
    #[error("Gas cost overflow")]
    GasCostOverflow,
    #[error("Offset overflow")]
    OffsetOverflow,
    #[error("Creation cost is too high")]
    CreationCostIsTooHigh,
    #[error("Max gas limit exceeded")]
    MaxGasLimitExceeded,
    #[error("Gas refunds underflow")]
    GasRefundsUnderflow,
    #[error("Gas refunds overflow")]
    GasRefundsOverflow,
    #[error("Initcode size exceeded")]
    InitcodeSizeExceeded,
    #[error("Priority fee greater than max fee per gas")]
    PriorityGreaterThanMaxFeePerGas,
    #[error("Intrinsic gas too low")]
    IntrinsicGasTooLow,
    #[error("Gas allowance exceeded")]
    GasAllowanceExceeded,
    #[error("Insufficient max fee per gas")]
    InsufficientMaxFeePerGas,
    #[error("Insufficient max fee per blob gas")]
    InsufficientMaxFeePerBlobGas,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InternalError {
    #[error("Tried to access last call frame but found none")]
    CouldNotAccessLastCallframe, // Last callframe before execution is the same as the first, but after execution the last callframe is actually the initial CF
    #[error("Tried to read from empty code")]
    TriedToIndexEmptyCode,
    #[error("Failed computing CREATE address")]
    CouldNotComputeCreateAddress,
    #[error("Failed computing CREATE2 address")]
    CouldNotComputeCreate2Address,
    #[error("Tried to slice non-existing data")]
    SlicingError,
    #[error("Could not pop callframe")]
    CouldNotPopCallframe,
    #[error("Account not found")]
    AccountNotFound,
    #[error("ExcessBlobGas should not be None")]
    ExcessBlobGasShouldNotBeNone,
    #[error("Error in utils file")]
    UtilsError,
    #[error("Accound should have been cached")]
    AccountShouldHaveBeenCached,
    #[error("Tried to convert one type to another")]
    ConversionError,
    #[error("Division error")]
    DivisionError,
    #[error("Overflow error")]
    OperationOverflow,
}

impl VMError {
    pub fn is_internal(&self) -> bool {
        matches!(self, VMError::Internal(_))
    }
}

#[derive(Debug, Clone)]
pub enum OpcodeSuccess {
    Continue,
    Result(ResultReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultReason {
    Stop,
    Revert,
    Return,
    SelfDestruct,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxResult {
    Success,
    Revert(VMError),
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
    /// Function to add gas to report, if it exceeds max gas limit it should return OutOfGas error. Only used for adding gas after execution.
    pub fn add_gas_with_max(&mut self, gas: u64, max: u64) -> Result<(), VMError> {
        self.gas_used = self
            .gas_used
            .checked_add(gas)
            .ok_or(VMError::Internal(InternalError::OperationOverflow))?;
        if self.gas_used > max {
            Err(VMError::OutOfGas)
        } else {
            Ok(())
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self.result, TxResult::Success)
    }
}
