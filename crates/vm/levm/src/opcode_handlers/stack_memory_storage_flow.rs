use crate::{constants::WORD_SIZE, vm::StorageSlot};

use super::*;

// Stack, Memory, Storage and Flow Operations (15)
// Opcodes: POP, MLOAD, MSTORE, MSTORE8, SLOAD, SSTORE, JUMP, JUMPI, PC, MSIZE, GAS, JUMPDEST, TLOAD, TSTORE, MCOPY

impl VM {
    // POP operation
    pub fn op_pop(&mut self, current_call_frame: &mut CallFrame) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::POP > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        current_call_frame.stack.pop()?;
        self.increase_gas(current_call_frame, gas_cost::POP);

        Ok(OpcodeSuccess::Continue)
    }

    // TLOAD operation
    pub fn op_tload(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::TLOAD > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let key = current_call_frame.stack.pop()?;
        let value = current_call_frame
            .transient_storage
            .get(&(current_call_frame.msg_sender, key))
            .cloned()
            .unwrap_or(U256::zero());

        current_call_frame.stack.push(value)?;
        self.increase_gas(current_call_frame, gas_cost::TLOAD);

        Ok(OpcodeSuccess::Continue)
    }

    // TSTORE operation
    pub fn op_tstore(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::TSTORE > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let key = current_call_frame.stack.pop()?;
        let value = current_call_frame.stack.pop()?;

        current_call_frame
            .transient_storage
            .insert((current_call_frame.msg_sender, key), value);
        self.increase_gas(current_call_frame, gas_cost::TSTORE);

        Ok(OpcodeSuccess::Continue)
    }

    // MLOAD operation
    pub fn op_mload(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let offset = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);
        let memory_expansion_cost = current_call_frame.memory.expansion_cost(offset + WORD_SIZE);
        let gas_cost = gas_cost::MLOAD_STATIC + memory_expansion_cost as u64;

        if current_call_frame.gas_used + gas_cost > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let value = current_call_frame.memory.load(offset);
        current_call_frame.stack.push(value)?;
        self.increase_gas(current_call_frame, gas_cost);

        Ok(OpcodeSuccess::Continue)
    }

    // MSTORE operation
    pub fn op_mstore(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let offset = current_call_frame.stack.pop()?.try_into().unwrap();
        let memory_expansion_cost = current_call_frame.memory.expansion_cost(offset + WORD_SIZE);
        let gas_cost = gas_cost::MSTORE_STATIC + memory_expansion_cost as u64;

        if current_call_frame.gas_used + gas_cost > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let value = current_call_frame.stack.pop()?;
        let mut value_bytes = [0u8; WORD_SIZE];
        value.to_big_endian(&mut value_bytes);

        current_call_frame.memory.store_bytes(offset, &value_bytes);
        self.increase_gas(current_call_frame, gas_cost);

        Ok(OpcodeSuccess::Continue)
    }

    // MSTORE8 operation
    pub fn op_mstore8(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let offset = current_call_frame.stack.pop()?.try_into().unwrap();
        let memory_expansion_cost = current_call_frame.memory.expansion_cost(offset + 1);
        let gas_cost = gas_cost::MSTORE8_STATIC + memory_expansion_cost as u64;

        if current_call_frame.gas_used + gas_cost > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let value = current_call_frame.stack.pop()?;
        let mut value_bytes = [0u8; WORD_SIZE];
        value.to_big_endian(&mut value_bytes);

        current_call_frame
            .memory
            .store_bytes(offset, value_bytes[WORD_SIZE - 1..WORD_SIZE].as_ref());
        self.increase_gas(current_call_frame, gas_cost);

        Ok(OpcodeSuccess::Continue)
    }

    // SLOAD operation
    pub fn op_sload(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let key = current_call_frame.stack.pop()?;
        let address = if let Some(delegate) = current_call_frame.delegate {
            delegate
        } else {
            current_call_frame.code_address
        };

        let current_value = self
            .db
            .read_account_storage(&address, &key)
            .unwrap_or_default()
            .current_value;

        current_call_frame.stack.push(current_value)?;

        Ok(OpcodeSuccess::Continue)
    }

    // SSTORE operation
    pub fn op_sstore(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.is_static {
            return Err(VMError::OpcodeNotAllowedInStaticContext);
        }

        let key = current_call_frame.stack.pop()?;
        let value = current_call_frame.stack.pop()?;
        let address = if let Some(delegate) = current_call_frame.delegate {
            delegate
        } else {
            current_call_frame.code_address
        };

        let slot = self.db.read_account_storage(&address, &key);
        let (original_value, _) = match slot {
            Some(slot) => (slot.original_value, slot.current_value),
            None => (value, value),
        };

        self.db.write_account_storage(
            &address,
            key,
            StorageSlot {
                original_value,
                current_value: value,
                is_cold: false,
            },
        );

        Ok(OpcodeSuccess::Continue)
    }

    // MSIZE operation
    pub fn op_msize(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::MSIZE > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        current_call_frame
            .stack
            .push(current_call_frame.memory.size())?;
        self.increase_gas(current_call_frame, gas_cost::MSIZE);

        Ok(OpcodeSuccess::Continue)
    }

    // GAS operation
    pub fn op_gas(&mut self, current_call_frame: &mut CallFrame) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::GAS > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let remaining_gas = current_call_frame.gas_limit - current_call_frame.gas_used - gas_cost::GAS;
        current_call_frame.stack.push(remaining_gas.into())?;
        self.increase_gas(current_call_frame, gas_cost::GAS);

        Ok(OpcodeSuccess::Continue)
    }

    // MCOPY operation
    pub fn op_mcopy(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let dest_offset = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);
        let src_offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);
        let size: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);

        let words_copied = (size + WORD_SIZE - 1) / WORD_SIZE;
        let memory_byte_size = (src_offset + size).max(dest_offset + size);
        let memory_expansion_cost = current_call_frame.memory.expansion_cost(memory_byte_size);
        let gas_cost = gas_cost::MCOPY_STATIC
            + gas_cost::MCOPY_DYNAMIC_BASE * words_copied as u64
            + memory_expansion_cost as u64;

        if current_call_frame.gas_used + gas_cost > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        self.increase_gas(current_call_frame, gas_cost);

        if size > 0 {
            current_call_frame
                .memory
                .copy(src_offset, dest_offset, size);
        }

        Ok(OpcodeSuccess::Continue)
    }

    // JUMP operation
    pub fn op_jump(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::JUMP > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let jump_address = current_call_frame.stack.pop()?;

        if !current_call_frame.jump(jump_address) {
            current_call_frame.gas_used = current_call_frame.gas_limit; // Mark gas limit consumed
            return Err(VMError::InvalidJump);
        }

        self.increase_gas(current_call_frame, gas_cost::JUMP);

        Ok(OpcodeSuccess::Continue)
    }

    // JUMPI operation
    pub fn op_jumpi(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let jump_address = current_call_frame.stack.pop()?;
        let condition = current_call_frame.stack.pop()?;

        if condition != U256::zero() && !current_call_frame.jump(jump_address) {
            current_call_frame.gas_used = current_call_frame.gas_limit; // Mark gas limit consumed
            return Err(VMError::InvalidJump);
        }

        self.increase_gas(current_call_frame, gas_cost::JUMPI);

        Ok(OpcodeSuccess::Continue)
    }

    // JUMPDEST operation
    pub fn op_jumpdest(&mut self, current_call_frame: &mut CallFrame) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::JUMPDEST > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        self.increase_gas(current_call_frame, gas_cost::JUMPDEST);

        Ok(OpcodeSuccess::Continue)
    }

    // PC operation
    pub fn op_pc(&mut self, current_call_frame: &mut CallFrame) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::PC > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        current_call_frame
            .stack
            .push(U256::from(current_call_frame.pc - 1))?;
        // self.increase_gas(current_call_frame, gas_cost::PC;
        self.increase_gas(current_call_frame, gas_cost::PC);

        Ok(OpcodeSuccess::Continue)
    }
}
