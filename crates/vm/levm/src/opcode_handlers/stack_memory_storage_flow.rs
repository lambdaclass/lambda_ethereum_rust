use crate::{
    account::StorageSlot,
    call_frame::CallFrame,
    constants::{call_opcode::WARM_ADDRESS_ACCESS_COST, COLD_STORAGE_ACCESS_COST, WORD_SIZE},
    errors::{InternalError, OpcodeSuccess, OutOfGasError, VMError},
    gas_cost::{
        self, mcopy_gas_cost, mload_gas_cost, mstore8_gas_cost, mstore_gas_cost, sstore_gas_cost,
    },
    vm::VM,
};
use ethereum_rust_core::{H256, U256};

// Stack, Memory, Storage and Flow Operations (15)
// Opcodes: POP, MLOAD, MSTORE, MSTORE8, SLOAD, SSTORE, JUMP, JUMPI, PC, MSIZE, GAS, JUMPDEST, TLOAD, TSTORE, MCOPY

impl VM {
    // POP operation
    pub fn op_pop(&mut self, current_call_frame: &mut CallFrame) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::POP)?;
        current_call_frame.stack.pop()?;
        Ok(OpcodeSuccess::Continue)
    }

    // TLOAD operation
    pub fn op_tload(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::TLOAD)?;

        let key = current_call_frame.stack.pop()?;
        let value = current_call_frame
            .transient_storage
            .get(&(current_call_frame.msg_sender, key))
            .cloned()
            .unwrap_or(U256::zero());

        current_call_frame.stack.push(value)?;
        Ok(OpcodeSuccess::Continue)
    }

    // TSTORE operation
    pub fn op_tstore(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::TSTORE)?;

        let key = current_call_frame.stack.pop()?;
        let value = current_call_frame.stack.pop()?;
        current_call_frame
            .transient_storage
            .insert((current_call_frame.msg_sender, key), value);

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

        let gas_cost =
            mload_gas_cost(current_call_frame, offset).map_err(|e| VMError::OutOfGasErr(e))?;

        self.increase_consumed_gas(current_call_frame, gas_cost)?;

        let value = current_call_frame.memory.load(offset)?;
        current_call_frame.stack.push(value)?;

        Ok(OpcodeSuccess::Continue)
    }

    // MSTORE operation
    pub fn op_mstore(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_err| VMError::VeryLargeNumber)?;

        let gas_cost =
            mstore_gas_cost(current_call_frame, offset).map_err(|e| VMError::OutOfGasErr(e))?;

        self.increase_consumed_gas(current_call_frame, gas_cost)?;

        let value = current_call_frame.stack.pop()?;
        let mut value_bytes = [0u8; WORD_SIZE];
        value.to_big_endian(&mut value_bytes);

        current_call_frame
            .memory
            .store_bytes(offset, &value_bytes)?;

        Ok(OpcodeSuccess::Continue)
    }

    // MSTORE8 operation
    pub fn op_mstore8(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        // TODO: modify expansion cost to accept U256
        let offset: usize = current_call_frame.stack.pop()?.try_into().unwrap();

        let gas_cost =
            mstore8_gas_cost(current_call_frame, offset).map_err(|e| VMError::OutOfGasErr(e))?;

        self.increase_consumed_gas(current_call_frame, gas_cost)?;

        let value = current_call_frame.stack.pop()?;
        let mut value_bytes = [0u8; WORD_SIZE];
        value.to_big_endian(&mut value_bytes);

        current_call_frame
            .memory
            .store_bytes(offset, value_bytes[WORD_SIZE - 1..WORD_SIZE].as_ref())?;

        Ok(OpcodeSuccess::Continue)
    }

    // SLOAD operation
    pub fn op_sload(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let key = current_call_frame.stack.pop()?;

        let address = current_call_frame.to;

        let mut bytes = [0u8; 32];
        key.to_big_endian(&mut bytes);
        let key = H256::from(bytes);

        let is_cached = self.cache.is_slot_cached(&address, key);

        let gas_cost = if is_cached {
            // If slot is warm (cached) add 100 to gas_cost
            WARM_ADDRESS_ACCESS_COST
        } else {
            // If slot is cold (not cached) add 2100 to gas_cost
            COLD_STORAGE_ACCESS_COST
        };

        let current_value = if is_cached {
            self.cache
                .get_storage_slot(address, key)
                .expect("Should be already cached") // Because entered the if is_slot_cached
                .current_value
        } else {
            self.get_storage_slot(&address, key).current_value
        };

        self.increase_consumed_gas(current_call_frame, gas_cost)?;

        current_call_frame.stack.push(current_value)?;
        Ok(OpcodeSuccess::Continue)
    }

    // SSTORE operation
    // TODO: https://github.com/lambdaclass/lambda_ethereum_rust/issues/1087
    pub fn op_sstore(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.is_static {
            return Err(VMError::OpcodeNotAllowedInStaticContext);
        }

        let key = current_call_frame.stack.pop()?;
        let value = current_call_frame.stack.pop()?;

        let mut bytes = [0u8; 32];
        key.to_big_endian(&mut bytes);
        let key = H256::from(bytes);

        let address = current_call_frame.to;

        let (gas_cost, storage_slot) =
            sstore_gas_cost(self, address, key, value).map_err(|e| VMError::OutOfGasErr(e))?;

        self.increase_consumed_gas(current_call_frame, gas_cost)?;

        self.cache.write_account_storage(
            &address,
            key,
            StorageSlot {
                original_value: storage_slot.original_value,
                current_value: value,
            },
        );

        Ok(OpcodeSuccess::Continue)
    }

    // MSIZE operation
    pub fn op_msize(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::MSIZE)?;
        current_call_frame
            .stack
            .push(current_call_frame.memory.size())?;
        Ok(OpcodeSuccess::Continue)
    }

    // GAS operation
    pub fn op_gas(&mut self, current_call_frame: &mut CallFrame) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::GAS)?;

        let remaining_gas = self
            .env
            .gas_limit
            .checked_sub(self.env.consumed_gas)
            .ok_or(VMError::OutOfGasErr(OutOfGasError::ConsumedGasOverflow))?
            .checked_sub(gas_cost::GAS)
            .ok_or(VMError::OutOfGasErr(OutOfGasError::ConsumedGasOverflow))?;
        // Note: These are not consumed gas calculations, but are related, so I used this wrapping here
        current_call_frame.stack.push(remaining_gas)?;

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

        let gas_cost = mcopy_gas_cost(current_call_frame, size, src_offset, dest_offset)
            .map_err(|e| VMError::OutOfGasErr(e))?;

        self.increase_consumed_gas(current_call_frame, gas_cost)?;

        if size > 0 {
            current_call_frame
                .memory
                .copy(src_offset, dest_offset, size)?;
        }

        Ok(OpcodeSuccess::Continue)
    }

    // JUMP operation
    pub fn op_jump(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::JUMP)?;

        let jump_address = current_call_frame.stack.pop()?;
        if !current_call_frame.jump(jump_address) {
            return Err(VMError::InvalidJump);
        }

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
            return Err(VMError::InvalidJump);
        }

        self.increase_consumed_gas(current_call_frame, gas_cost::JUMPI)?;
        Ok(OpcodeSuccess::Continue)
    }

    // JUMPDEST operation
    pub fn op_jumpdest(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::JUMPDEST)?;
        Ok(OpcodeSuccess::Continue)
    }

    // PC operation
    pub fn op_pc(&mut self, current_call_frame: &mut CallFrame) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::PC)?;

        current_call_frame.stack.push(U256::from(
            current_call_frame
                .pc
                .checked_sub(1)
                .ok_or(VMError::Internal(InternalError::PCUnderflowed))?,
        ))?;

        Ok(OpcodeSuccess::Continue)
    }
}
