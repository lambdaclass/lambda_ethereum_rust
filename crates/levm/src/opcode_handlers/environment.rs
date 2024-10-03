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
        Ok(())
    }

    pub fn op_calldatasize(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_calldatacopy(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
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
        Ok(())
    }

    pub fn op_returndatacopy(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_extcodehash(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }
}
