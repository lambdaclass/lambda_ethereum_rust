use super::*;

// Stack, Memory, Storage and Flow Operations (15)
// Opcodes: POP, MLOAD, MSTORE, MSTORE8, SLOAD, SSTORE, JUMP, JUMPI, PC, MSIZE, GAS, JUMPDEST, TLOAD, TSTORE, MCOPY

impl VM {
    pub fn op_pop(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_mload(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_mstore(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_mstore8(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_sload(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_sstore(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_jump(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_jumpi(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_pc(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_msize(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_gas(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_jumpdest(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_tload(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_tstore(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_mcopy(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }
}
