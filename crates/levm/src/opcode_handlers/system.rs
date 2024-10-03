use super::*;

// System Operations (10)
// Opcodes: CREATE, CALL, CALLCODE, RETURN, DELEGATECALL, CREATE2, STATICCALL, REVERT, INVALID, SELFDESTRUCT

impl VM {
    pub fn op_create(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_call(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_callcode(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_return(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_delegatecall(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_create2(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_staticcall(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_revert(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_invalid(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_selfdestruct(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }
}
