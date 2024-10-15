use crate::constants::WORD_SIZE;

use super::*;

// Environmental Information (16)
// Opcodes: ADDRESS, BALANCE, ORIGIN, CALLER, CALLVALUE, CALLDATALOAD, CALLDATASIZE, CALLDATACOPY, CODESIZE, CODECOPY, GASPRICE, EXTCODESIZE, EXTCODECOPY, RETURNDATASIZE, RETURNDATACOPY, EXTCODEHASH

impl VM {
    pub fn get_value_from_calldata(calldata: &Bytes, offset: usize) -> U256 {
        let mut buffer = [0u8; 32];
    
        let available_len = calldata.len().saturating_sub(offset);
        let copy_len = available_len.min(32); 
    
        if copy_len > 0 {
            buffer[..copy_len].copy_from_slice(&calldata[offset..offset + copy_len]);
        }
    
        U256::from_big_endian(&buffer)
    }

    // CALLDATALOAD operation
    pub fn op_calldataload(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::CALLDATALOAD > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);
        let value = Self::get_value_from_calldata(&current_call_frame.calldata, offset);
        current_call_frame.stack.push(value)?;
        self.env.consumed_gas += gas_cost::CALLDATALOAD;

        Ok(OpcodeSuccess::Continue)
    }

    // CALLDATASIZE operation
    pub fn op_calldatasize(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::CALLDATASIZE > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        current_call_frame
            .stack
            .push(U256::from(current_call_frame.calldata.len()))?;
        self.env.consumed_gas += gas_cost::CALLDATASIZE;

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
            .unwrap_or(usize::MAX);
        let calldata_offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);
        let size: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);

        let minimum_word_size = (size + WORD_SIZE - 1) / WORD_SIZE;
        let memory_expansion_cost =
            current_call_frame.memory.expansion_cost(dest_offset + size) as u64;
        let gas_cost = gas_cost::CALLDATACOPY_STATIC
            + gas_cost::CALLDATACOPY_DYNAMIC_BASE * minimum_word_size as u64
            + memory_expansion_cost;

        if self.env.consumed_gas + gas_cost > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        self.env.consumed_gas += gas_cost;

        if size == 0 {
            return Ok(OpcodeSuccess::Continue);
        }

        let data = current_call_frame
            .calldata
            .slice(calldata_offset..calldata_offset + size);
        current_call_frame.memory.store_bytes(dest_offset, &data);

        Ok(OpcodeSuccess::Continue)
    }

    // RETURNDATASIZE operation
    pub fn op_returndatasize(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::RETURNDATASIZE > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        current_call_frame
            .stack
            .push(U256::from(current_call_frame.returndata.len()))?;
        self.env.consumed_gas += gas_cost::RETURNDATASIZE;

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
            .unwrap_or(usize::MAX);
        let returndata_offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);
        let size: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);

        let minimum_word_size = (size + WORD_SIZE - 1) / WORD_SIZE;
        let memory_expansion_cost =
            current_call_frame.memory.expansion_cost(dest_offset + size) as u64;
        let gas_cost = gas_cost::RETURNDATACOPY_STATIC
            + gas_cost::RETURNDATACOPY_DYNAMIC_BASE * minimum_word_size as u64
            + memory_expansion_cost;

        if self.env.consumed_gas + gas_cost > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        self.env.consumed_gas += gas_cost;

        if size == 0 {
            return Ok(OpcodeSuccess::Continue);
        }

        let data = current_call_frame
            .returndata
            .slice(returndata_offset..returndata_offset + size);
        current_call_frame.memory.store_bytes(dest_offset, &data);

        Ok(OpcodeSuccess::Continue)
    }
}
