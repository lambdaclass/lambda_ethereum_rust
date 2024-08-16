use bytes::Bytes;
use ethereum_rust_core::Address;
use revm::primitives::result::Output as RevmOutput;
use revm::primitives::result::SuccessReason as RevmSuccessReason;
use revm::primitives::ExecutionResult as RevmExecutionResult;

#[derive(Debug)]
pub enum ExecutionResult {
    Success {
        reason: SuccessReason,
        gas_used: u64,
        gas_refunded: u64,
        output: Output,
    },
    /// Reverted by `REVERT` opcode
    Revert { gas_used: u64, output: Bytes },
    /// Reverted for other reasons, spends all gas.
    Halt {
        reason: String,
        /// Halting will spend all the gas, which will be equal to gas_limit.
        gas_used: u64,
    },
}

#[derive(Debug)]
pub enum SuccessReason {
    Stop,
    Return,
    SelfDestruct,
    EofReturnContract,
}

#[derive(Debug)]
pub enum Output {
    Call(Bytes),
    Create(Bytes, Option<Address>),
}

impl From<RevmExecutionResult> for ExecutionResult {
    fn from(val: RevmExecutionResult) -> Self {
        match val {
            RevmExecutionResult::Success {
                reason,
                gas_used,
                gas_refunded,
                logs: _,
                output,
            } => ExecutionResult::Success {
                reason: match reason {
                    RevmSuccessReason::Stop => SuccessReason::Stop,
                    RevmSuccessReason::Return => SuccessReason::Return,
                    RevmSuccessReason::SelfDestruct => SuccessReason::SelfDestruct,
                    RevmSuccessReason::EofReturnContract => SuccessReason::EofReturnContract,
                },
                gas_used,
                gas_refunded,
                output: match output {
                    RevmOutput::Call(bytes) => Output::Call(bytes.0),
                    RevmOutput::Create(bytes, addr) => Output::Create(
                        bytes.0,
                        addr.map(|addr| Address::from_slice(addr.0.as_ref())),
                    ),
                },
            },
            RevmExecutionResult::Revert { gas_used, output } => ExecutionResult::Revert {
                gas_used,
                output: output.0,
            },
            RevmExecutionResult::Halt { reason, gas_used } => ExecutionResult::Halt {
                reason: format!("{:?}", reason),
                gas_used,
            },
        }
    }
}

impl ExecutionResult {
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            ExecutionResult::Success {
                reason: _,
                gas_used: _,
                gas_refunded: _,
                output: _
            }
        )
    }
}
