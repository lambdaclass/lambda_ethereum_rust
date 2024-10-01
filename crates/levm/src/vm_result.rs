use bytes::Bytes;

use crate::call_frame::Log;

/// Main EVM error.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VMError {
    /// Transaction validation error.
    StackError(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResultReason {
    Stop,
    Return,
    StackUnderflow,
    StackOverflow,
    InvalidJump,
    OpcodeNotAllowedInStaticContext,
}

/// Result of a transaction execution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExecutionResult {
    Success {
        reason: ResultReason,
        logs: Vec<Log>,
        return_data: Bytes,
    },

    Halt {
        reason: ResultReason,
    },
}

impl ExecutionResult {
    pub fn is_success(&self) -> bool {
        matches!(self, ExecutionResult::Success { .. })
    }

    pub fn is_halt(&self) -> bool {
        matches!(self, ExecutionResult::Halt { .. })
    }
}
