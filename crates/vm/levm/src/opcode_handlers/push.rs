// Push Operations
// Opcodes: PUSH0, PUSH1 ... PUSH32
use super::*;

impl VM {
    // PUSH operation
    pub fn op_push(
        &mut self,
        current_call_frame: &mut CallFrame,
        op: Opcode,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::PUSHN > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let n_bytes = (op as u8) - (Opcode::PUSH1 as u8) + 1;

        let next_n_bytes = current_call_frame
            .bytecode
            .get(current_call_frame.pc()..current_call_frame.pc() + n_bytes as usize)
            .ok_or(VMError::InvalidBytecode)?; // This shouldn't happen during execution

        let value_to_push = U256::from(next_n_bytes);

        current_call_frame.stack.push(value_to_push)?;

        current_call_frame.increment_pc_by(n_bytes as usize);

        self.increase_gas(current_call_frame, gas_cost::PUSHN);

        Ok(OpcodeSuccess::Continue)
    }

    // PUSH0
    pub fn op_push0(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::PUSH0 > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        current_call_frame.stack.push(U256::zero())?;
        self.increase_gas(current_call_frame, gas_cost::PUSH0);

        Ok(OpcodeSuccess::Continue)
    }
}
