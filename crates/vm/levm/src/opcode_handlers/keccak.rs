use crate::{
    call_frame::CallFrame,
    errors::{OpcodeSuccess, VMError},
    gas_cost::keccak256_gas_cost,
    vm::VM,
};
use ethereum_rust_core::U256;
use sha3::{Digest, Keccak256};

// KECCAK256 (1)
// Opcodes: KECCAK256

impl VM {
    pub fn op_keccak256(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let offset = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);
        let size = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);

        let gas_cost = keccak256_gas_cost(current_call_frame, size, offset)
            .map_err(|e| VMError::OutOfGasErr(e))?;

        self.increase_consumed_gas(current_call_frame, gas_cost)?;

        let value_bytes = current_call_frame.memory.load_range(offset, size)?;

        let mut hasher = Keccak256::new();
        hasher.update(value_bytes);
        let result = hasher.finalize();
        current_call_frame
            .stack
            .push(U256::from_big_endian(&result))?;

        Ok(OpcodeSuccess::Continue)
    }
}
