use super::*;

// Environmental Information (16)
// Opcodes: ADDRESS, BALANCE, ORIGIN, CALLER, CALLVALUE, CALLDATALOAD, CALLDATASIZE, CALLDATACOPY, CODESIZE, CODECOPY, GASPRICE, EXTCODESIZE, EXTCODECOPY, RETURNDATASIZE, RETURNDATACOPY, EXTCODEHASH

impl VM {
    pub fn op_address(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_balance(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_origin(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_caller(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_callvalue(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_calldataload(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);
        let value = U256::from_big_endian(&current_call_frame.calldata.slice(offset..offset + 32));
        current_call_frame.stack.push(value)?;
        Ok(())
    }

    pub fn op_calldatasize(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        current_call_frame
            .stack
            .push(U256::from(current_call_frame.calldata.len()))?;
        Ok(())
    }

    pub fn op_calldatacopy(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let dest_offset = current_call_frame
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
        if size == 0 {
            return Ok(()); // Return early for zero size
        }
        let data = current_call_frame
            .calldata
            .slice(calldata_offset..calldata_offset + size);

        current_call_frame.memory.store_bytes(dest_offset, &data);
        Ok(())
    }

    pub fn op_codesize(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_codecopy(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_gasprice(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_extcodesize(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_extcodecopy(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_returndatasize(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        current_call_frame
            .stack
            .push(U256::from(current_call_frame.returndata.len()))?;
        Ok(())
    }

    pub fn op_returndatacopy(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let dest_offset = current_call_frame
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
        if size == 0 {
            return Ok(()); // Return early for zero size
        }
        let data = current_call_frame
            .returndata
            .slice(returndata_offset..returndata_offset + size);
        current_call_frame.memory.store_bytes(dest_offset, &data);
        Ok(())
    }

    pub fn op_extcodehash(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }
}
