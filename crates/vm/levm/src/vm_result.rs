use bytes::Bytes;

use crate::call_frame::Log;

/// Errors that halts the program
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VMError {
    StackUnderflow,
    StackOverflow,
    InvalidJump,
    OpcodeNotAllowedInStaticContext,
    OpcodeNotFound,
    InvalidBytecode,
    OutOfGas,
    FatalError, // this should never really happen
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResultReason {
    Stop,
    Return,
}

/// Result of a transaction execution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExecutionResult {
    Success {
        reason: ResultReason,
        logs: Vec<Log>,
        return_data: Bytes,
    },
    /// Reverted by `REVERT` opcode that doesn't spend all gas.
    Revert {
        reason: VMError,
        gas_used: u64,
        output: Bytes,
    },
    /// Reverted for various reasons and spend all gas.
    Halt {
        reason: VMError,
        /// Halting will spend all the gas, and will be equal to gas_limit.
        gas_used: u64,
    },
}

impl ExecutionResult {
    pub fn logs(&self) -> &[Log] {
        match self {
            ExecutionResult::Success { logs, .. } => logs,
            ExecutionResult::Revert { .. } => &[],
            ExecutionResult::Halt { .. } => &[],
        }
    }
}
