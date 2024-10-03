// Exchange Operations (16)
// Opcodes: SWAP1 ... SWAP16
use super::*;

impl VM {
    pub fn op_swap(current_call_frame: &mut CallFrame, op: Opcode) -> Result<(), VMError> {
        let depth = (op as u8) - (Opcode::SWAP1 as u8) + 1;

        if current_call_frame.stack.len() < depth as usize {
            return Err(VMError::StackUnderflow);
        }
        let stack_top_index = current_call_frame.stack.len();
        let to_swap_index = stack_top_index
            .checked_sub(depth as usize)
            .ok_or(VMError::StackUnderflow)?;
        current_call_frame
            .stack
            .swap(stack_top_index - 1, to_swap_index - 1);
        Ok(())
    }
    
}
