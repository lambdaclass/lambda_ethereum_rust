// Exchange Operations (16)
// Opcodes: SWAP1 ... SWAP16
use super::*;

impl VM {
    // SWAP operation
    pub fn op_swap(&mut self, current_call_frame: &mut CallFrame, op: Opcode) -> Result<OpcodeSuccess, VMError> {
        // Determine the depth based on the opcode
        let depth = (op as u8) - (Opcode::SWAP1 as u8) + 1;

        // Check for gas consumption
        if self.env.consumed_gas + gas_cost::SWAPN > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        // Ensure the stack has enough elements to swap
        if current_call_frame.stack.len() < depth as usize {
            return Err(VMError::StackUnderflow);
        }

        // Index of the top of the stack and the index to swap with
        let stack_top_index = current_call_frame.stack.len();
        let to_swap_index = stack_top_index.checked_sub(depth as usize).ok_or(VMError::StackUnderflow)?;

        // Perform the swap
        current_call_frame.stack.swap(stack_top_index - 1, to_swap_index);

        // Update the consumed gas
        self.env.consumed_gas += gas_cost::SWAPN;

        Ok(OpcodeSuccess::Continue)
    }
}
