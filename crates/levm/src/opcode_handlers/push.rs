// Push Operations
// Opcodes: PUSH0, PUSH1 ... PUSH32
use super::*;

impl VM {
    pub fn op_push(&self, current_call_frame: &mut CallFrame, op: Opcode) -> Result<(), VMError> {
        let n_bytes = (op as u8) - (Opcode::PUSH1 as u8) + 1;
        let next_n_bytes = current_call_frame
            .bytecode
            .get(current_call_frame.pc()..current_call_frame.pc() + n_bytes as usize)
            .ok_or(VMError::InvalidBytecode)?; // this shouldn't really happen during execution
        let value_to_push = U256::from(next_n_bytes);
        current_call_frame.stack.push(value_to_push)?;
        current_call_frame.increment_pc_by(n_bytes as usize);
        Ok(())
    }
}
