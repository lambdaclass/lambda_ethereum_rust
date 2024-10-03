// KECCAK256 (1)
// Opcodes: KECCAK256
use super::*;
use sha3::{Digest, Keccak256};

impl VM {
    pub fn op_keccak256(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let offset = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);
        let size = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);
        let value_bytes = current_call_frame.memory.load_range(offset, size);

        let mut hasher = Keccak256::new();
        hasher.update(value_bytes);
        let result = hasher.finalize();
        current_call_frame
            .stack
            .push(U256::from_big_endian(&result))?;
        Ok(())
    }
}
