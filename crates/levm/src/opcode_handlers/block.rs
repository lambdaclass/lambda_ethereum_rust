use crate::block::{BlockEnv, LAST_AVAILABLE_BLOCK_LIMIT};

// Block Information (11)
// Opcodes: BLOCKHASH, COINBASE, TIMESTAMP, NUMBER, PREVRANDAO, GASLIMIT, CHAINID, SELFBALANCE, BASEFEE, BLOBHASH, BLOBBASEFEE
use super::*;

impl VM {
    pub fn op_blockhash(&self, current_call_frame: &mut CallFrame, block_env: &BlockEnv) -> Result<(), VMError> {
        let block_number = current_call_frame.stack.pop()?;

        // If number is not in the valid range (last 256 blocks), return zero.
        if block_number
            < block_env
                .number
                .saturating_sub(U256::from(LAST_AVAILABLE_BLOCK_LIMIT))
            || block_number >= block_env.number
        {
            current_call_frame.stack.push(U256::zero())?;
            return Ok(());
        }

        if let Some(block_hash) = self.db.get(&block_number) {
            current_call_frame
                .stack
                .push(U256::from_big_endian(&block_hash.0))?;
        } else {
            current_call_frame.stack.push(U256::zero())?;
        };        
        Ok(())
    }

    pub fn op_coinbase(&self, current_call_frame: &mut CallFrame, block_env: &BlockEnv) -> Result<(), VMError> {
        let coinbase = block_env.coinbase;
        current_call_frame.stack.push(address_to_word(coinbase))?;
        Ok(())
    }

    pub fn op_timestamp(&self, current_call_frame: &mut CallFrame, block_env: &BlockEnv) -> Result<(), VMError> {
        let timestamp = block_env.timestamp;
        current_call_frame.stack.push(timestamp)?;
        Ok(())
    }

    pub fn op_number(&self, current_call_frame: &mut CallFrame, block_env: &BlockEnv) -> Result<(), VMError> {
        let block_number = block_env.number;
        current_call_frame.stack.push(block_number)?;
        Ok(())
    }

    pub fn op_prevrandao(&self, current_call_frame: &mut CallFrame, block_env: &BlockEnv) -> Result<(), VMError> {
        let randao = block_env.prev_randao.unwrap_or_default();
        current_call_frame
            .stack
            .push(U256::from_big_endian(randao.0.as_slice()))?;
        Ok(())
    }

    pub fn op_gaslimit(&self, current_call_frame: &mut CallFrame, block_env: &BlockEnv) -> Result<(), VMError> {
        let gas_limit = block_env.gas_limit;
        current_call_frame.stack.push(U256::from(gas_limit))?;
        Ok(())
    }

    pub fn op_chainid(&self, current_call_frame: &mut CallFrame, block_env: &BlockEnv) -> Result<(), VMError> {
        let chain_id = block_env.chain_id;
        current_call_frame.stack.push(U256::from(chain_id))?;
        Ok(())
    }

    pub fn op_selfbalance(&self, current_call_frame: &mut CallFrame, block_env: &BlockEnv) -> Result<(), VMError> {
        todo!("when we have accounts implemented");
    }

    pub fn op_basefee(&self, current_call_frame: &mut CallFrame, block_env: &BlockEnv) -> Result<(), VMError> {
        let base_fee = block_env.base_fee_per_gas;
        current_call_frame.stack.push(base_fee)?;
        Ok(())
    }

    pub fn op_blobhash(&self, current_call_frame: &mut CallFrame, block_env: &BlockEnv) -> Result<(), VMError> {
        todo!("when we have tx implemented");
    }

    pub fn op_blobbasefee(&self, current_call_frame: &mut CallFrame, block_env: &BlockEnv) -> Result<(), VMError> {
        let blob_base_fee = block_env.calculate_blob_gas_price();
        current_call_frame.stack.push(blob_base_fee)?;
        Ok(())
    }
}
