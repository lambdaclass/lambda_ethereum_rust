use crate::{
    call_frame::CallFrame,
    errors::{OpcodeSuccess, VMError},
    gas_cost, memory,
    vm::VM,
};
use ethrex_core::U256;
use sha3::{Digest, Keccak256};

// KECCAK256 (1)
// Opcodes: KECCAK256

impl VM {
    pub fn op_keccak256(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;
        let size: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;

        let gas_cost =
            gas_cost::keccak256(current_call_frame, size, offset).map_err(VMError::OutOfGas)?;

        self.increase_consumed_gas(current_call_frame, gas_cost)?;

        let mut hasher = Keccak256::new();
        hasher.update(memory::load_range(
            &mut current_call_frame.memory,
            offset,
            size,
        )?);
        current_call_frame
            .stack
            .push(U256::from_big_endian(&hasher.finalize()))?;

        Ok(OpcodeSuccess::Continue)
    }
}
