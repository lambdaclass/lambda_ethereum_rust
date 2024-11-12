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
    SenderAccountShouldNotHaveBytecode,
    #[error("Sender balance should contain transfer value")]
    SenderBalanceShouldContainTransferValue,
    #[error("Gas price is lower than base fee")]
    GasPriceIsLowerThanBaseFee,
    #[error("Address already occupied")]
    AddressAlreadyOccupied,
    #[error("Contract output too big")]
    ContractOutputTooBig,
    #[error("Invalid initial byte")]
    InvalidInitialByte,
    #[error("Nonce overflow")]
    NonceOverflow,
    #[error("Memory load out of bounds")]
    MemoryLoadOutOfBounds,
    #[error("Memory store out of bounds")]
    MemoryStoreOutOfBounds,
    #[error("Gas limit price product overflow")]
    GasLimitPriceProductOverflow,
    #[error("Slicing error")]
    SlicingError,
    #[error("Indexing error")]
    IndexingError,
    #[error("Internal error: {0}")]
    Internal(#[from] InternalError),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InternalError {
    #[error("Could not access last callframe")]
    CouldNotAccessLastCallframe,
    #[error("Could not pop callframe")]
    CouldNotPopCallframe,
    #[error("Account not found")]
    AccountNotFound,
    #[error("ExcessBlobGas should not be None")]
    ExcessBlobGasShouldNotBeNone,
    #[error("Error in utils file")]
    UtilsError,
}

pub enum OpcodeSuccess {
    Continue,
    Result(ResultReason),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
