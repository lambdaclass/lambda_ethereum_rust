use crate::{
    call_frame::CallFrame,
    errors::{InternalError, OpcodeSuccess, VMError},
    gas_cost,
    opcodes::Opcode,
    vm::VM,
};
use ethereum_rust_core::U256;

// Push Operations
// Opcodes: PUSH0, PUSH1 ... PUSH32

impl VM {
    // PUSH operation
    pub fn op_push(
        &mut self,
        current_call_frame: &mut CallFrame,
        op: Opcode,
    ) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::PUSHN)?;

        let n_bytes = (usize::from(op))
            .checked_sub(usize::from(Opcode::PUSH1))
            .ok_or(VMError::InvalidOpcode)?
            .checked_add(1)
            .ok_or(VMError::InvalidOpcode)?;

        let mut readed_n_bytes: Vec<u8> = current_call_frame
            .bytecode
            .get(current_call_frame.pc()..)
            .ok_or(VMError::InvalidBytecode)?
            .iter()
            .take(n_bytes)
            .cloned()
            .collect();

        // If I have fewer bytes to read than I need, I add as many leading 0s as necessary
        if readed_n_bytes.len() < n_bytes {
            let gap_to_fill =
                n_bytes
                    .checked_sub(readed_n_bytes.len())
                    .ok_or(VMError::Internal(
                        InternalError::ArithmeticOperationUnderflow,
                    ))?;
            let padding = vec![0; gap_to_fill];
            readed_n_bytes.splice(0..0, padding);
        }

        let bytes_push: &[u8] = &readed_n_bytes;

        current_call_frame.stack.push(U256::from(bytes_push))?;

        current_call_frame.increment_pc_by(n_bytes)?;

        Ok(OpcodeSuccess::Continue)
    }

    // PUSH0
    pub fn op_push0(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::PUSH0)?;

        current_call_frame.stack.push(U256::zero())?;

        Ok(OpcodeSuccess::Continue)
    }
}
