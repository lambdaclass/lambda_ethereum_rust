use crate::{
    account::StorageSlot,
    call_frame::CallFrame,
    constants::{
        call_opcode::WARM_ADDRESS_ACCESS_COST, gas_cost, COLD_STORAGE_ACCESS_COST, WORD_SIZE,
    },
    errors::{OpcodeSuccess, VMError},
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
        let offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;
        let memory_expansion_cost = current_call_frame.memory.expansion_cost(
            offset
                .checked_add(WORD_SIZE)
                .ok_or(VMError::OverflowInArithmeticOp)?,
        )?;
        let gas_cost = gas_cost::MLOAD_STATIC + memory_expansion_cost;

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
        let memory_expansion_cost = current_call_frame.memory.expansion_cost(
            offset
                .checked_add(WORD_SIZE)
                .ok_or(VMError::OverflowInArithmeticOp)?,
        )?;
        let gas_cost = gas_cost::MSTORE_STATIC + memory_expansion_cost;

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
        let offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;
        let memory_expansion_cost = current_call_frame.memory.expansion_cost(
            offset
                .checked_add(1)
                .ok_or(VMError::OverflowInArithmeticOp)?,
        )?;
        let gas_cost = gas_cost::MSTORE8_STATIC + memory_expansion_cost;

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

        let mut base_dynamic_gas: U256 = U256::zero();

        let current_value = if self.cache.is_slot_cached(&address, key) {
            // If slot is warm (cached) add 100 to base_dynamic_gas
            base_dynamic_gas += WARM_ADDRESS_ACCESS_COST;

            self.get_storage_slot(&address, key).current_value
        } else {
            // If slot is cold (not cached) add 2100 to base_dynamic_gas
            base_dynamic_gas += COLD_STORAGE_ACCESS_COST;

            self.get_storage_slot(&address, key).current_value
        };

        self.increase_consumed_gas(current_call_frame, base_dynamic_gas)?;

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

        // Convert key from U256 to H256
        let mut bytes = [0u8; 32];
        key.to_big_endian(&mut bytes);
        let key = H256::from(bytes);

        let address = current_call_frame.to;

        let mut base_dynamic_gas: U256 = U256::zero();

        if !self.cache.is_slot_cached(&address, key) {
            // If slot is cold 2100 is added to base_dynamic_gas
            base_dynamic_gas += U256::from(2100);
        };

        let storage_slot = self.get_storage_slot(&address, key);

        base_dynamic_gas += if value == storage_slot.current_value {
            U256::from(100)
        } else if storage_slot.current_value == storage_slot.original_value {
            if storage_slot.original_value == U256::zero() {
                U256::from(20000)
            } else {
                U256::from(2900)
            }
        } else {
            U256::from(100)
        };

        self.increase_consumed_gas(current_call_frame, base_dynamic_gas)?;

        // Gas Refunds
        // TODO: Think about what to do in case of underflow of gas refunds (when we try to substract from it if the value is low)
        let mut gas_refunds = U256::zero();
        if value != storage_slot.current_value {
            if storage_slot.current_value == storage_slot.original_value {
                if storage_slot.original_value.is_zero() && value.is_zero() {
                    gas_refunds = gas_refunds
                        .checked_add(U256::from(4800))
                        .ok_or(VMError::GasRefundsOverflow)?;
                }
            } else if !storage_slot.original_value.is_zero() {
                if storage_slot.current_value.is_zero() {
                    gas_refunds = gas_refunds
                        .checked_sub(U256::from(4800))
                        .ok_or(VMError::GasRefundsUnderflow)?;
                } else if value.is_zero() {
                    gas_refunds = gas_refunds
                        .checked_add(U256::from(4800))
                        .ok_or(VMError::GasRefundsOverflow)?;
                }
            } else if value == storage_slot.original_value {
                if storage_slot.original_value.is_zero() {
                    gas_refunds = gas_refunds
                        .checked_add(U256::from(19900))
                        .ok_or(VMError::GasRefundsOverflow)?;
                } else {
                    gas_refunds = gas_refunds
                        .checked_add(U256::from(2800))
                        .ok_or(VMError::GasRefundsOverflow)?;
                }
            }
        };

        self.env.refunded_gas = self
            .env
            .refunded_gas
            .checked_add(gas_refunds)
            .ok_or(VMError::GasLimitPriceProductOverflow)?;

        self.cache.write_account_storage(
            &address,
            key,
            StorageSlot {
                original_value: storage_slot.original_value,
                current_value: value,
            },
        )?;

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

        let remaining_gas = self.env.gas_limit - self.env.consumed_gas - gas_cost::GAS;
        current_call_frame.stack.push(remaining_gas)?;

        Ok(OpcodeSuccess::Continue)
    }

    // MCOPY operation
    pub fn op_mcopy(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let dest_offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;
        let src_offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;
        let size: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;

        let words_copied = (size + WORD_SIZE - 1) / WORD_SIZE;

        let memory_byte_size = src_offset
            .checked_add(size)
            .and_then(|src_sum| {
                dest_offset
                    .checked_add(size)
                    .map(|dest_sum| src_sum.max(dest_sum))
            })
            .ok_or(VMError::OverflowInArithmeticOp)?;

        let memory_expansion_cost = current_call_frame.memory.expansion_cost(memory_byte_size)?;
        let gas_cost = gas_cost::MCOPY_STATIC
            + gas_cost::MCOPY_DYNAMIC_BASE * words_copied
            + memory_expansion_cost;

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
        if !current_call_frame.jump(jump_address)? {
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

        if condition != U256::zero() && !current_call_frame.jump(jump_address)? {
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

        current_call_frame
            .stack
            .push(U256::from(current_call_frame.pc - 1))?;

        Ok(OpcodeSuccess::Continue)
    }
}
