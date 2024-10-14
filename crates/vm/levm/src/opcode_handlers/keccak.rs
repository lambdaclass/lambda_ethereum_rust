use crate::constants::WORD_SIZE;

// KECCAK256 (1)
// Opcodes: KECCAK256
use super::*;
use sha3::{Digest, Keccak256};

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

        let minimum_word_size = (size + WORD_SIZE - 1) / WORD_SIZE;
        let memory_expansion_cost = current_call_frame.memory.expansion_cost(offset + size);
        let gas_cost = gas_cost::KECCAK25_STATIC
            + gas_cost::KECCAK25_DYNAMIC_BASE * minimum_word_size as u64
            + memory_expansion_cost as u64;
        if current_call_frame.gas_used + gas_cost > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let value_bytes = current_call_frame.memory.load_range(offset, size);

        let mut hasher = Keccak256::new();
        hasher.update(value_bytes);
        let result = hasher.finalize();
        current_call_frame
            .stack
            .push(U256::from_big_endian(&result))?;
        self.increase_gas(current_call_frame, gas_cost);

        Ok(OpcodeSuccess::Continue)
    }
}
