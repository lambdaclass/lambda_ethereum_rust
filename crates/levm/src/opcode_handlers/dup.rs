// Duplication Operation (16)
// Opcodes: DUP1 ... DUP16
use super::*;

impl VM {
    // DUP operation
    pub fn op_dup(current_call_frame: &mut CallFrame, op: Opcode) -> Result<(), VMError> {
        let depth = (op as u8) - (Opcode::DUP1 as u8) + 1;

        if current_call_frame.stack.len() < depth as usize {
            return Err(VMError::StackUnderflow);
        }

        let value_at_depth = current_call_frame
            .stack
            .get(current_call_frame.stack.len() - depth as usize)?;
        current_call_frame.stack.push(*value_at_depth)?;
        Ok(())
    }
}
