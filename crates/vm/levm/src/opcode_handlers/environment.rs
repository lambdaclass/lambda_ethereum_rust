use crate::{
    call_frame::CallFrame,
    constants::{
        call_opcode::{COLD_ADDRESS_ACCESS_COST, WARM_ADDRESS_ACCESS_COST},
        gas_cost, WORD_SIZE,
    },
    errors::{InternalError, OpcodeSuccess, VMError},
    vm::{word_to_address, VM},
};
use bytes::Bytes;
use ethereum_rust_core::U256;
use sha3::{Digest, Keccak256};

// Environmental Information (16)
// Opcodes: ADDRESS, BALANCE, ORIGIN, CALLER, CALLVALUE, CALLDATALOAD, CALLDATASIZE, CALLDATACOPY, CODESIZE, CODECOPY, GASPRICE, EXTCODESIZE, EXTCODECOPY, RETURNDATASIZE, RETURNDATACOPY, EXTCODEHASH

impl VM {
    // ADDRESS operation
    pub fn op_address(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::ADDRESS)?;

        let addr = current_call_frame.to; // The recipient of the current call.

        current_call_frame.stack.push(U256::from(addr.as_bytes()))?;

        Ok(OpcodeSuccess::Continue)
    }

    // BALANCE operation
    pub fn op_balance(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let address = &word_to_address(current_call_frame.stack.pop()?);

        if self.cache.is_account_cached(address) {
            self.increase_consumed_gas(current_call_frame, WARM_ADDRESS_ACCESS_COST)?;
        } else {
            self.increase_consumed_gas(current_call_frame, COLD_ADDRESS_ACCESS_COST)?;
            self.cache_from_db(address);
        };

        let balance = self.get_account(address).info.balance;

        current_call_frame.stack.push(balance)?;
        Ok(OpcodeSuccess::Continue)
    }

    // ORIGIN operation
    pub fn op_origin(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::ORIGIN)?;

        let origin = self.env.origin;
        current_call_frame
            .stack
            .push(U256::from(origin.as_bytes()))?;

        Ok(OpcodeSuccess::Continue)
    }

    // CALLER operation
    pub fn op_caller(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::CALLER)?;

        let caller = current_call_frame.msg_sender;
        current_call_frame
            .stack
            .push(U256::from(caller.as_bytes()))?;

        Ok(OpcodeSuccess::Continue)
    }

    // CALLVALUE operation
    pub fn op_callvalue(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::CALLVALUE)?;

        let callvalue = current_call_frame.msg_value;

        current_call_frame.stack.push(callvalue)?;

        Ok(OpcodeSuccess::Continue)
    }

    // CALLDATALOAD operation
    pub fn op_calldataload(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::CALLDATALOAD)?;

        let offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;

        // All bytes after the end of the calldata are set to 0.
        let mut data = [0u8; 32];
        for (i, byte) in current_call_frame
            .calldata
            .iter()
            .skip(offset)
            .take(32)
            .enumerate()
        {
            if let Some(data_byte) = data.get_mut(i) {
                *data_byte = *byte;
            }
        }
        let result = U256::from_big_endian(&data);

        current_call_frame.stack.push(result)?;

        Ok(OpcodeSuccess::Continue)
    }

    // CALLDATASIZE operation
    pub fn op_calldatasize(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::CALLDATASIZE)?;

        current_call_frame
            .stack
            .push(U256::from(current_call_frame.calldata.len()))?;

        Ok(OpcodeSuccess::Continue)
    }

    // CALLDATACOPY operation
    pub fn op_calldatacopy(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let dest_offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_err| VMError::VeryLargeNumber)?;
        let calldata_offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_err| VMError::VeryLargeNumber)?;
        let size: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_err| VMError::VeryLargeNumber)?;

        let minimum_word_size = (size + WORD_SIZE - 1) / WORD_SIZE;
        let memory_expansion_cost = current_call_frame.memory.expansion_cost(
            dest_offset
                .checked_add(size)
                .ok_or(VMError::MemoryLoadOutOfBounds)?,
        )?;
        let gas_cost = gas_cost::CALLDATACOPY_STATIC
            + gas_cost::CALLDATACOPY_DYNAMIC_BASE * minimum_word_size
            + memory_expansion_cost;

        self.increase_consumed_gas(current_call_frame, gas_cost)?;

        if size == 0 {
            return Ok(OpcodeSuccess::Continue);
        }

        let mut data = [0u8; 32];
        for (i, byte) in current_call_frame
            .calldata
            .iter()
            .skip(calldata_offset)
            .take(32)
            .enumerate()
        {
            if let Some(data_byte) = data.get_mut(i) {
                *data_byte = *byte;
            }
        }

        current_call_frame.memory.store_bytes(dest_offset, &data)?;

        Ok(OpcodeSuccess::Continue)
    }

    // CODESIZE operation
    pub fn op_codesize(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::CODESIZE > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        current_call_frame
            .stack
            .push(U256::from(current_call_frame.bytecode.len()))?;

        self.increase_consumed_gas(current_call_frame, gas_cost::CODESIZE)?;

        Ok(OpcodeSuccess::Continue)
    }

    // CODECOPY operation
    pub fn op_codecopy(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let dest_offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;
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

        let minimum_word_size = (size + WORD_SIZE - 1) / WORD_SIZE;

        let memory_expansion_cost = current_call_frame.memory.expansion_cost(
            dest_offset
                .checked_add(size)
                .ok_or(VMError::MemoryLoadOutOfBounds)?,
        )?;

        let gas_cost = gas_cost::CODECOPY_STATIC
            + gas_cost::CODECOPY_DYNAMIC_BASE * minimum_word_size
            + memory_expansion_cost;

        self.increase_consumed_gas(current_call_frame, gas_cost)?;

        let bytecode_len = current_call_frame.bytecode.len();
        let code = if offset < bytecode_len {
            current_call_frame.bytecode.slice(
                offset
                    ..(offset
                        .checked_add(size)
                        .ok_or(VMError::MemoryLoadOutOfBounds)?)
                    .min(bytecode_len),
            )
        } else {
            vec![0u8; size].into()
        };

        current_call_frame.memory.store_bytes(dest_offset, &code)?;

        Ok(OpcodeSuccess::Continue)
    }

    // GASPRICE operation
    pub fn op_gasprice(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::GASPRICE)?;

        current_call_frame.stack.push(self.env.gas_price)?;

        Ok(OpcodeSuccess::Continue)
    }

    // EXTCODESIZE operation
    pub fn op_extcodesize(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let address = word_to_address(current_call_frame.stack.pop()?);

        if self.cache.is_account_cached(&address) {
            self.increase_consumed_gas(current_call_frame, WARM_ADDRESS_ACCESS_COST)?;
        } else {
            self.increase_consumed_gas(current_call_frame, COLD_ADDRESS_ACCESS_COST)?;
            self.cache_from_db(&address);
        };

        let bytecode = self.get_account(&address).info.bytecode;

        current_call_frame.stack.push(bytecode.len().into())?;
        Ok(OpcodeSuccess::Continue)
    }

    // EXTCODECOPY operation
    pub fn op_extcodecopy(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let address = word_to_address(current_call_frame.stack.pop()?);
        let dest_offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;
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

        let minimum_word_size = (size + WORD_SIZE - 1) / WORD_SIZE;
        let memory_expansion_cost = current_call_frame.memory.expansion_cost(
            dest_offset
                .checked_add(size)
                .ok_or(VMError::MemoryLoadOutOfBounds)?,
        )?;
        let gas_cost =
            gas_cost::EXTCODECOPY_DYNAMIC_BASE * minimum_word_size + memory_expansion_cost;

        if self.cache.is_account_cached(&address) {
            self.increase_consumed_gas(current_call_frame, gas_cost + WARM_ADDRESS_ACCESS_COST)?;
        } else {
            self.increase_consumed_gas(current_call_frame, gas_cost + COLD_ADDRESS_ACCESS_COST)?;
            self.cache_from_db(&address);
        };

        let mut bytecode = self.get_account(&address).info.bytecode;

        if bytecode.len()
            < offset
                .checked_add(size)
                .ok_or(VMError::MemoryLoadOutOfBounds)?
        {
            let mut extended_code = bytecode.to_vec();
            extended_code.resize(
                offset
                    .checked_add(size)
                    .ok_or(VMError::MemoryLoadOutOfBounds)?,
                0,
            );
            bytecode = Bytes::from(extended_code);
        }
        current_call_frame.memory.store_bytes(
            dest_offset,
            bytecode
                .get(
                    offset
                        ..offset
                            .checked_add(size)
                            .ok_or(VMError::MemoryLoadOutOfBounds)?,
                )
                .ok_or(VMError::Internal(InternalError::SlicingError))?, // bytecode can be "refactored" in order to avoid handling the error.
        )?;

        Ok(OpcodeSuccess::Continue)
    }

    // RETURNDATASIZE operation
    pub fn op_returndatasize(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        self.increase_consumed_gas(current_call_frame, gas_cost::RETURNDATASIZE)?;

        current_call_frame
            .stack
            .push(U256::from(current_call_frame.sub_return_data.len()))?;

        Ok(OpcodeSuccess::Continue)
    }

    // RETURNDATACOPY operation
    pub fn op_returndatacopy(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let dest_offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;
        let returndata_offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;
        let size: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;

        let minimum_word_size = (size + WORD_SIZE - 1) / WORD_SIZE;
        let memory_expansion_cost = current_call_frame.memory.expansion_cost(
            dest_offset
                .checked_add(size)
                .ok_or(VMError::MemoryLoadOutOfBounds)?,
        )?;
        let gas_cost = gas_cost::RETURNDATACOPY_STATIC
            + gas_cost::RETURNDATACOPY_DYNAMIC_BASE * minimum_word_size
            + memory_expansion_cost;

        self.increase_consumed_gas(current_call_frame, gas_cost)?;

        if size == 0 {
            return Ok(OpcodeSuccess::Continue);
        }

        let sub_return_data_len = current_call_frame.sub_return_data.len();
        let data = if returndata_offset < sub_return_data_len {
            current_call_frame.sub_return_data.slice(
                returndata_offset
                    ..(returndata_offset
                        .checked_add(size)
                        .ok_or(VMError::MemoryLoadOutOfBounds)?)
                    .min(sub_return_data_len),
            )
        } else {
            vec![0u8; size].into()
        };

        current_call_frame.memory.store_bytes(dest_offset, &data)?;

        Ok(OpcodeSuccess::Continue)
    }

    // EXTCODEHASH operation
    pub fn op_extcodehash(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let address = word_to_address(current_call_frame.stack.pop()?);

        if self.cache.is_account_cached(&address) {
            self.increase_consumed_gas(current_call_frame, WARM_ADDRESS_ACCESS_COST)?;
        } else {
            self.increase_consumed_gas(current_call_frame, COLD_ADDRESS_ACCESS_COST)?;
            self.cache_from_db(&address);
        };

        let bytecode = self.get_account(&address).info.bytecode;

        let mut hasher = Keccak256::new();
        hasher.update(bytecode);
        let result = hasher.finalize();
        current_call_frame
            .stack
            .push(U256::from_big_endian(&result))?;

        Ok(OpcodeSuccess::Continue)
    }
}
