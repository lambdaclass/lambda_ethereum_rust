use super::*;

// Stack, Memory, Storage and Flow Operations (15)
// Opcodes: POP, MLOAD, MSTORE, MSTORE8, SLOAD, SSTORE, JUMP, JUMPI, PC, MSIZE, GAS, JUMPDEST, TLOAD, TSTORE, MCOPY

impl VM {
    pub fn op_pop(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        current_call_frame.stack.pop()?;
        Ok(())
    }

    pub fn op_mload(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let offset = current_call_frame.stack.pop()?.try_into().unwrap_or(usize::MAX);
        let value = current_call_frame.memory.load(offset);
        current_call_frame.stack.push(value)?;
        Ok(())
    }

    pub fn op_mstore(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let offset = current_call_frame.stack.pop()?.try_into().unwrap_or(usize::MAX);
        let value = current_call_frame.stack.pop()?;
        let mut value_bytes = [0u8; 32];
        value.to_big_endian(&mut value_bytes);
        current_call_frame.memory.store_bytes(offset, &value_bytes);
        Ok(())
    }

    pub fn op_mstore8(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let offset = current_call_frame.stack.pop()?.try_into().unwrap_or(usize::MAX);
        let value = current_call_frame.stack.pop()?;
        let mut value_bytes = [0u8; 32];
        value.to_big_endian(&mut value_bytes);
        current_call_frame.memory.store_bytes(offset, value_bytes[31..32].as_ref());
        Ok(())
    }

    pub fn op_msize(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        current_call_frame.stack.push(current_call_frame.memory.size())?;
        Ok(())
    }

    pub fn op_mcopy(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let dest_offset = current_call_frame.stack.pop()?.try_into().unwrap_or(usize::MAX);
        let src_offset = current_call_frame.stack.pop()?.try_into().unwrap_or(usize::MAX);
        let size = current_call_frame.stack.pop()?.try_into().unwrap_or(usize::MAX);
        if size == 0 {
            return Ok(());
        }
        current_call_frame.memory.copy(src_offset, dest_offset, size);
        Ok(())
    }

    pub fn op_tload(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let key = current_call_frame.stack.pop()?;
        let value = current_call_frame
            .transient_storage
            .get(&(current_call_frame.msg_sender, key))
            .cloned()
            .unwrap_or(U256::zero());

        current_call_frame.stack.push(value)?;
        Ok(())
    }

    pub fn op_tstore(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let key = current_call_frame.stack.pop()?;
        let value = current_call_frame.stack.pop()?;

        current_call_frame
            .transient_storage
            .insert((current_call_frame.msg_sender, key), value);
        Ok(())
    }

    pub fn op_sload(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_sstore(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_jump(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let jump_address = current_call_frame.stack.pop()?;
        if !current_call_frame.jump(jump_address) {
            return Err(VMError::InvalidJump);
        }
        Ok(())
    }

    pub fn op_jumpi(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let jump_address = current_call_frame.stack.pop()?;
        let condition = current_call_frame.stack.pop()?;
        if condition != U256::zero() && !current_call_frame.jump(jump_address) {
            return Err(VMError::InvalidJump);
        }
        Ok(())
    }

    pub fn op_pc(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        current_call_frame
            .stack
            .push(U256::from(current_call_frame.pc - 1))?;
        Ok(())
    }

    pub fn op_gas(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_jumpdest(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }
}
