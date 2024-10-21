use crate::block::LAST_AVAILABLE_BLOCK_LIMIT;

// Block Information (11)
// Opcodes: BLOCKHASH, COINBASE, TIMESTAMP, NUMBER, PREVRANDAO, GASLIMIT, CHAINID, SELFBALANCE, BASEFEE, BLOBHASH, BLOBBASEFEE
use super::*;

impl VM {
    // BLOCKHASH operation
    pub fn op_blockhash(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::BLOCKHASH > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let block_number = current_call_frame.stack.pop()?;
        self.env.consumed_gas += gas_cost::BLOCKHASH;

        // If the block number is not valid, return zero
        if block_number
            < self
                .env
                .block_number
                .saturating_sub(LAST_AVAILABLE_BLOCK_LIMIT)
            || block_number >= self.env.block_number
        {
            current_call_frame.stack.push(U256::zero())?;
            return Ok(OpcodeSuccess::Continue);
        }

        if let Some(block_hash) = self.db.block_hashes.get(&block_number) {
            current_call_frame
                .stack
                .push(U256::from_big_endian(&block_hash.0))?;
        } else {
            current_call_frame.stack.push(U256::zero())?;
        }

        Ok(OpcodeSuccess::Continue)
    }

    // COINBASE operation
    pub fn op_coinbase(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::COINBASE > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }
        current_call_frame
            .stack
            .push(address_to_word(self.env.coinbase))?;
        self.env.consumed_gas += gas_cost::COINBASE;

        Ok(OpcodeSuccess::Continue)
    }

    // TIMESTAMP operation
    pub fn op_timestamp(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::TIMESTAMP > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }
        current_call_frame.stack.push(self.env.timestamp)?;
        self.env.consumed_gas += gas_cost::TIMESTAMP;

        Ok(OpcodeSuccess::Continue)
    }

    // NUMBER operation
    pub fn op_number(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::NUMBER > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        current_call_frame.stack.push(self.env.block_number)?;
        self.env.consumed_gas += gas_cost::NUMBER;

        Ok(OpcodeSuccess::Continue)
    }

    // PREVRANDAO operation
    pub fn op_prevrandao(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::PREVRANDAO > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }
        let randao = self.env.prev_randao.unwrap_or_default(); // Assuming block_env has been integrated
        current_call_frame
            .stack
            .push(U256::from_big_endian(randao.0.as_slice()))?;
        self.env.consumed_gas += gas_cost::PREVRANDAO;

        Ok(OpcodeSuccess::Continue)
    }

    // GASLIMIT operation
    pub fn op_gaslimit(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::GASLIMIT > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }
        current_call_frame.stack.push(self.env.gas_limit)?;
        self.env.consumed_gas += gas_cost::GASLIMIT;

        Ok(OpcodeSuccess::Continue)
    }

    // CHAINID operation
    pub fn op_chainid(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::CHAINID > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }
        current_call_frame.stack.push(self.env.chain_id)?;
        self.env.consumed_gas += gas_cost::CHAINID;

        Ok(OpcodeSuccess::Continue)
    }

    // SELFBALANCE operation
    pub fn op_selfbalance(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::SELFBALANCE > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let balance = self.db.balance(&current_call_frame.code_address);
        current_call_frame.stack.push(balance)?;

        self.env.consumed_gas += gas_cost::SELFBALANCE;

        Ok(OpcodeSuccess::Continue)
    }

    // BASEFEE operation
    pub fn op_basefee(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::BASEFEE > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }
        current_call_frame.stack.push(self.env.base_fee_per_gas)?;
        self.env.consumed_gas += gas_cost::BASEFEE;

        Ok(OpcodeSuccess::Continue)
    }

    // BLOBHASH operation
    pub fn op_blobhash(
        &mut self,
        _current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::BLOBHASH > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        self.env.consumed_gas += gas_cost::BLOBHASH;

        unimplemented!("when we have tx implemented");

        // Ok(OpcodeSuccess::Continue)
    }

    // BLOBBASEFEE operation
    pub fn op_blobbasefee(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::BLOBBASEFEE > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }
        // TODO: Calculate blob gas price.
        let blob_base_fee = U256::zero();
        current_call_frame.stack.push(blob_base_fee)?;
        self.env.consumed_gas += gas_cost::BLOBBASEFEE;

        Ok(OpcodeSuccess::Continue)
    }
}

use std::str::FromStr;
fn address_to_word(address: Address) -> U256 {
    // This unwrap can't panic, as Address are 20 bytes long and U256 use 32 bytes
    U256::from_str(&format!("{address:?}")).unwrap()
}
