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
        gas_used: u64,
        gas_refunded: u64,
        logs: Vec<Log>,
    },
    Revert {
        reason: ResultReason,
        gas_used: u64,
    },
    Halt {
        reason: ResultReason,
        /// Halting will spend all the gas, and will be equal to gas_limit.
        gas_used: u64,
    },
}
