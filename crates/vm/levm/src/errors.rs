use crate::account::Account;
use bytes::Bytes;
use ethereum_rust_core::{types::Log, Address};
use std::collections::HashMap;
use thiserror;

/// Errors that halt the program
#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
pub enum VMError {
    #[error("Stack Underflow")]
    StackUnderflow,
    #[error("Stack Overflow")]
    StackOverflow,
    #[error("Invalid Jump")]
    InvalidJump,
    #[error("Opcode Not Allowed In Static Context")]
    OpcodeNotAllowedInStaticContext,
    #[error("Opcode Not Found")]
    OpcodeNotFound,
    #[error("Invalid Bytecode")]
    InvalidBytecode,
    #[error("Out Of Gas")]
    OutOfGas,
    #[error("Very Large Number")]
    VeryLargeNumber,
    #[error("Fatal Error")]
    FatalError,
    #[error("Invalid Transaction")]
    InvalidTransaction,
    #[error("Revert Opcode")]
    RevertOpcode,
    #[error("Invalid Opcode")]
    InvalidOpcode,
    #[error("Missing Blob Hashes")]
    MissingBlobHashes,
    #[error("Blob Hash Index Out Of Bounds")]
    BlobHashIndexOutOfBounds,
    #[error("Sender Account Does Not Exist")]
    SenderAccountDoesNotExist,
    #[error("Address Does Not Match An Account")]
    AddressDoesNotMatchAnAccount,
    #[error("Sender Account Should Not Have Bytecode")]
    SenderAccountShouldNotHaveBytecode,
    #[error("Sender Balance Should Contain Transfer Value")]
    SenderBalanceShouldContainTransferValue,
    #[error("Gas Price Is Lower Than Base Fee")]
    GasPriceIsLowerThanBaseFee,
    #[error("Address Already Occupied")]
    AddressAlreadyOccupied,
    #[error("Contract Output Too Big")]
    ContractOutputTooBig,
    #[error("Invalid Initial Byte")]
    InvalidInitialByte,
    #[error("Memory Load Out Of Bounds")]
    MemoryLoadOutOfBounds,
    #[error("Memory Store Out Of Bounds")]
    MemoryStoreOutOfBounds,
    #[error("Gas Limit Product Overflow")]
    GasLimitPriceProductOverflow,
    #[error("Balance Overflow")]
    BalanceOverflow,

    // OutOfGas?
    #[error("Balance Underflow")]
    BalanceUnderflow,

    #[error("Remaining Gas Underflow")]
    RemainingGasUnderflow,                // When gas used is higher than gas limit, is there already an error for that?

    // OutOfGas
    #[error("Gas Cost Overflow")]
    GasCostOverflow,
    #[error("Gas Used Overflow")]
    GasUsedOverflow,
    #[error("Creation Cost Is Too High")]
    CreationCostIsTooHigh,
    #[error("Consumed Gas Overflow")]
    ConsumedGasOverflow,
    // Internal
    #[error("Internal error: {0}")]
    Internal(#[from] InternalError),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, thiserror::Error)]
pub enum InternalError {
    #[error("Overflowed when incrementing nonce")]
    NonceOverflowed,
    #[error("Underflowed when incrementing nonce")]
    NonceUnderflowed,
    #[error("Overflowed when incrementing program counter")]
    PCOverflowed,
    #[error("Underflowed when decrementing program counter")]
    PCUnderflowed,
    #[error("Arithmetic operation overflowed")]
    ArithmeticOperationOverflow,
    #[error("Arithmetic operation underflowed")]
    ArithmeticOperationUnderflow,
    #[error("Arithmetic operation divided by zero")]
    ArithmeticOperationDividedByZero,
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
