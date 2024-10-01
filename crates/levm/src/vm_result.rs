use bytes::Bytes;
use ethereum_types::U256;

use crate::call_frame::Log;

/// Main EVM error.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VMError {
    /// Transaction validation error.
    StackError(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResultReason {
    Stop {},
    Return {},
}

/// Result of a transaction execution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExecutionResult {
    Success {
        reason: ResultReason,
        logs: Vec<Log>,
        return_data: Bytes,
    },
    Revert {
        reason: ResultReason,
        gas_used: U256,
        return_data: Bytes,
    },
    Halt {
        reason: ResultReason,
        /// Halting will spend all the gas, and will be equal to gas_limit.
        gas_used: U256,
    },
}
