use crate::{
    call_frame::CallFrame,
    gas_cost,
    constants::WORD_SIZE,
    errors::{InternalError, OpcodeSuccess, VMError},
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

        let minimum_word_size = (size
            .checked_add(WORD_SIZE)
            .ok_or(VMError::Internal(
                InternalError::ArithmeticOperationOverflow,
            ))?
            .saturating_sub(1))
            / WORD_SIZE;
        let memory_expansion_cost =
            current_call_frame
                .memory
                .expansion_cost(offset.checked_add(size).ok_or(VMError::Internal(
                    InternalError::ArithmeticOperationOverflow,
                ))?)?;
        let minimum_word_size_cost = gas_cost::KECCAK25_DYNAMIC_BASE
            .checked_mul(minimum_word_size.into())
            .ok_or(VMError::GasCostOverflow)?;

        let gas_cost = gas_cost::KECCAK25_STATIC
            .checked_add(minimum_word_size_cost)
            .ok_or(VMError::GasCostOverflow)?
            .checked_add(memory_expansion_cost)
            .ok_or(VMError::GasCostOverflow)?;

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
