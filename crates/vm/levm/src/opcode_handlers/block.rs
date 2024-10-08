use crate::{block::LAST_AVAILABLE_BLOCK_LIMIT, vm::word_to_address};

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
                .block
                .number
                .saturating_sub(U256::from(LAST_AVAILABLE_BLOCK_LIMIT))
            || block_number >= self.env.block.number
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

        let coinbase = self.env.block.coinbase; // Assuming block_env has been integrated
        current_call_frame.stack.push(address_to_word(coinbase))?;
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

        let timestamp = self.env.block.timestamp; // Assuming block_env has been integrated
        current_call_frame.stack.push(timestamp)?;
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

        let block_number = self.env.block.number; // Assuming block_env has been integrated
        current_call_frame.stack.push(block_number)?;
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

        let randao = self.env.block.prev_randao.unwrap_or_default(); // Assuming block_env has been integrated
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

        let gas_limit = self.env.block.gas_limit; // Assuming block_env has been integrated
        current_call_frame.stack.push(U256::from(gas_limit))?;
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

        let chain_id = self.env.block.chain_id; // Assuming block_env has been integrated
        current_call_frame.stack.push(U256::from(chain_id))?;
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

        let base_fee = self.env.block.base_fee_per_gas; // Assuming block_env has been integrated
        current_call_frame.stack.push(base_fee)?;
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

        let blob_base_fee = self.env.block.calculate_blob_gas_price(); // Assuming block_env has been integrated
        current_call_frame.stack.push(blob_base_fee)?;
        self.env.consumed_gas += gas_cost::BLOBBASEFEE;

        Ok(OpcodeSuccess::Continue)
    }

    // ADDRESS operation
    pub fn op_address(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::ADDRESS > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let addr = current_call_frame.code_address;

        current_call_frame.stack.push(U256::from(addr.as_bytes()))?;
        self.env.consumed_gas += gas_cost::ADDRESS;

        Ok(OpcodeSuccess::Continue)
    }

    // BALANCE operation
    pub fn op_balance(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::BALANCE > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let addr = current_call_frame.stack.pop()?;

        let balance = self.db.balance(&word_to_address(addr));
        current_call_frame.stack.push(balance)?;

        self.env.consumed_gas += gas_cost::BALANCE;

        Ok(OpcodeSuccess::Continue)
    }

    // ORIGIN operation
    pub fn op_origin(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::ORIGIN > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let origin = self.env.origin;
        current_call_frame
            .stack
            .push(U256::from(origin.as_bytes()))?;

        self.env.consumed_gas += gas_cost::ORIGIN;

        Ok(OpcodeSuccess::Continue)
    }

    // CALLER operation
    pub fn op_caller(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if self.env.consumed_gas + gas_cost::CALLER > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let caller = current_call_frame.msg_sender;
        current_call_frame
            .stack
            .push(U256::from(caller.as_bytes()))?;

        self.env.consumed_gas += gas_cost::CALLER;

        Ok(OpcodeSuccess::Continue)
    }

    // CALLVALUE operation
    pub fn op_callvalue(
        &mut self,
        _current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        unimplemented!();
    }

    // CODESIZE operation
    pub fn op_codesize(
        &mut self,
        _current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        unimplemented!();
    }

    // CODECOPY operation
    pub fn op_codecopy(
        &mut self,
        _current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        unimplemented!();
    }

    // GASPRICE operation
    pub fn op_gasprice(
        &mut self,
        _current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        unimplemented!();
    }

    // EXTCODESIZE operation
    pub fn op_extcodesize(
        &mut self,
        _current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        unimplemented!();
    }

    // EXTCODECOPY operation
    pub fn op_extcodecopy(
        &mut self,
        _current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        unimplemented!();
    }

    // EXTCODEHASH operation
    pub fn op_extcodehash(
        &mut self,
        _current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        unimplemented!();
    }
}

use std::str::FromStr;
fn address_to_word(address: Address) -> U256 {
    // This unwrap can't panic, as Address are 20 bytes long and U256 use 32 bytes
    U256::from_str(&format!("{address:?}")).unwrap()
}
