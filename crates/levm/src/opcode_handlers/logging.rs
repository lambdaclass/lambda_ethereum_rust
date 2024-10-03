// Logging Operations (5)
// Opcodes: LOG0 ... LOG4
use super::*;

// Implement empty op_log(n) method

impl VM {
    pub fn op_log(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }
}
