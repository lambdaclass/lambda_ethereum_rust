use crate::block::BlockEnv;

// Block Information (11)
// Opcodes: BLOCKHASH, COINBASE, TIMESTAMP, NUMBER, PREVRANDAO, GASLIMIT, CHAINID, SELFBALANCE, BASEFEE, BLOBHASH, BLOBBASEFEE
use super::*;

impl VM {
    pub fn op_blockhash(&self, current_call_frame: &mut CallFrame, block_env: BlockEnv) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_coinbase(&self, current_call_frame: &mut CallFrame, block_env: BlockEnv) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_timestamp(&self, current_call_frame: &mut CallFrame, block_env: BlockEnv) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_number(&self, current_call_frame: &mut CallFrame, block_env: BlockEnv) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_prevrandao(&self, current_call_frame: &mut CallFrame, block_env: BlockEnv) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_gaslimit(&self, current_call_frame: &mut CallFrame, block_env: BlockEnv) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_chainid(&self, current_call_frame: &mut CallFrame, block_env: BlockEnv) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_selfbalance(&self, current_call_frame: &mut CallFrame, block_env: BlockEnv) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_basefee(&self, current_call_frame: &mut CallFrame, block_env: BlockEnv) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_blobhash(&self, current_call_frame: &mut CallFrame, block_env: BlockEnv) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_blobbasefee(&self, current_call_frame: &mut CallFrame, block_env: BlockEnv) -> Result<(), VMError> {
        Ok(())
    }
}
