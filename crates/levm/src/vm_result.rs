use bytes::Bytes;

use crate::call_frame::Log;

/// Errors that halts the program
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VMError {
    StackUnderflow,
    StackOverflow,
    InvalidJump,
    OpcodeNotAllowedInStaticContext,
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
}

impl ExecutionResult {
    pub fn logs(&self) -> &[Log] {
        match self {
            ExecutionResult::Success { logs, .. } => logs,
        }
    }
}
