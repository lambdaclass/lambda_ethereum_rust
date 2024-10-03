// Push Operations
// Opcodes: PUSH0, PUSH1 ... PUSH32
use super::*;

// Implement op_push(n) method

impl VM {
    pub fn op_push(current_call_frame: &mut CallFrame, op: Opcode) -> Result<(), VMError> {
        Ok(())
    }
}
