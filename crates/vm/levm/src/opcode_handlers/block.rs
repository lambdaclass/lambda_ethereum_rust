use crate::{
    block::LAST_AVAILABLE_BLOCK_LIMIT,
    constants::{call_opcode, WORD_SIZE},
    vm::word_to_address,
};
use sha3::{Digest, Keccak256};

// Block Information (11)
// Opcodes: BLOCKHASH, COINBASE, TIMESTAMP, NUMBER, PREVRANDAO, GASLIMIT, CHAINID, SELFBALANCE, BASEFEE, BLOBHASH, BLOBBASEFEE
use super::*;

impl VM {
    // BLOCKHASH operation
    pub fn op_blockhash(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::BLOCKHASH > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let block_number = current_call_frame.stack.pop()?;
        self.increase_gas(current_call_frame, gas_cost::BLOCKHASH);

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
        if current_call_frame.gas_used + gas_cost::COINBASE > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let coinbase = self.env.block.coinbase; // Assuming block_env has been integrated
        current_call_frame.stack.push(address_to_word(coinbase))?;
        self.increase_gas(current_call_frame, gas_cost::COINBASE);

        Ok(OpcodeSuccess::Continue)
    }

    // TIMESTAMP operation
    pub fn op_timestamp(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::TIMESTAMP > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let timestamp = self.env.block.timestamp; // Assuming block_env has been integrated
        current_call_frame.stack.push(timestamp)?;
        self.increase_gas(current_call_frame, gas_cost::TIMESTAMP);

        Ok(OpcodeSuccess::Continue)
    }

    // NUMBER operation
    pub fn op_number(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::NUMBER > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let block_number = self.env.block.number; // Assuming block_env has been integrated
        current_call_frame.stack.push(block_number)?;
        self.increase_gas(current_call_frame, gas_cost::NUMBER);

        Ok(OpcodeSuccess::Continue)
    }

    // PREVRANDAO operation
    pub fn op_prevrandao(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::PREVRANDAO > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let randao = self.env.block.prev_randao.unwrap_or_default(); // Assuming block_env has been integrated
        current_call_frame
            .stack
            .push(U256::from_big_endian(randao.0.as_slice()))?;
        self.increase_gas(current_call_frame, gas_cost::PREVRANDAO);

        Ok(OpcodeSuccess::Continue)
    }

    // GASLIMIT operation
    pub fn op_gaslimit(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::GASLIMIT > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let gas_limit = self.env.block.gas_limit; // Assuming block_env has been integrated
        current_call_frame.stack.push(U256::from(gas_limit))?;
        self.increase_gas(current_call_frame, gas_cost::GASLIMIT);

        Ok(OpcodeSuccess::Continue)
    }

    // CHAINID operation
    pub fn op_chainid(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::CHAINID > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let chain_id = self.env.block.chain_id; // Assuming block_env has been integrated
        current_call_frame.stack.push(U256::from(chain_id))?;
        self.increase_gas(current_call_frame, gas_cost::CHAINID);

        Ok(OpcodeSuccess::Continue)
    }

    // SELFBALANCE operation
    pub fn op_selfbalance(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::SELFBALANCE > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let balance = self.db.balance(&current_call_frame.code_address);
        current_call_frame.stack.push(balance)?;

        self.increase_gas(current_call_frame, gas_cost::SELFBALANCE);

        Ok(OpcodeSuccess::Continue)
    }

    // BASEFEE operation
    pub fn op_basefee(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::BASEFEE > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let base_fee = self.env.block.base_fee_per_gas; // Assuming block_env has been integrated
        current_call_frame.stack.push(base_fee)?;
        self.increase_gas(current_call_frame, gas_cost::BASEFEE);

        Ok(OpcodeSuccess::Continue)
    }

    // BLOBHASH operation
    pub fn op_blobhash(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::BLOBHASH > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        self.increase_gas(current_call_frame, gas_cost::BLOBHASH);

        unimplemented!("when we have tx implemented");

        // Ok(OpcodeSuccess::Continue)
    }

    // BLOBBASEFEE operation
    pub fn op_blobbasefee(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::BLOBBASEFEE > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let blob_base_fee = self.env.block.calculate_blob_gas_price(); // Assuming block_env has been integrated
        current_call_frame.stack.push(blob_base_fee)?;
        self.increase_gas(current_call_frame, gas_cost::BLOBBASEFEE);

        Ok(OpcodeSuccess::Continue)
    }

    // ADDRESS operation
    pub fn op_address(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::ADDRESS > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let addr = if current_call_frame.delegate.is_some() {
            current_call_frame.msg_sender
        } else {
            current_call_frame.code_address
        };

        current_call_frame.stack.push(U256::from(addr.as_bytes()))?;
        self.increase_gas(current_call_frame, gas_cost::ADDRESS);

        Ok(OpcodeSuccess::Continue)
    }

    // BALANCE operation
    pub fn op_balance(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::BALANCE > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let addr = current_call_frame.stack.pop()?;

        let balance = self.db.balance(&word_to_address(addr));
        current_call_frame.stack.push(balance)?;

        self.increase_gas(current_call_frame, gas_cost::BALANCE);

        Ok(OpcodeSuccess::Continue)
    }

    // ORIGIN operation
    pub fn op_origin(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::ORIGIN > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let origin = self.env.origin;
        current_call_frame
            .stack
            .push(U256::from(origin.as_bytes()))?;

        self.increase_gas(current_call_frame, gas_cost::ORIGIN);

        Ok(OpcodeSuccess::Continue)
    }

    // CALLER operation
    pub fn op_caller(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::CALLER > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let caller = current_call_frame.msg_sender;
        current_call_frame
            .stack
            .push(U256::from(caller.as_bytes()))?;

        self.increase_gas(current_call_frame, gas_cost::CALLER);

        Ok(OpcodeSuccess::Continue)
    }

    // CALLVALUE operation
    pub fn op_callvalue(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::CALLVALUE > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let callvalue = current_call_frame.msg_value;

        current_call_frame.stack.push(callvalue)?;

        self.increase_gas(current_call_frame, gas_cost::CALLVALUE);

        Ok(OpcodeSuccess::Continue)
    }

    // CODESIZE operation
    pub fn op_codesize(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::CODESIZE > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        current_call_frame
            .stack
            .push(U256::from(current_call_frame.bytecode.len()))?;

        self.increase_gas(current_call_frame, gas_cost::CODESIZE);

        Ok(OpcodeSuccess::Continue)
    }

    // CODECOPY operation
    pub fn op_codecopy(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let dest_offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;
        let offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;
        let size: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;

        let minimum_word_size = (size + WORD_SIZE - 1) / WORD_SIZE;

        let memory_expansion_cost =
            current_call_frame.memory.expansion_cost(dest_offset + size) as u64;

        let gas_cost = gas_cost::CODECOPY_STATIC
            + gas_cost::CODECOPY_DYNAMIC_BASE * minimum_word_size as u64
            + memory_expansion_cost;

        if current_call_frame.gas_used + gas_cost > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let code = current_call_frame.bytecode.slice(offset..offset + size);

        current_call_frame.memory.store_bytes(dest_offset, &code);

        self.increase_gas(current_call_frame, gas_cost);

        Ok(OpcodeSuccess::Continue)
    }

    // GASPRICE operation
    pub fn op_gasprice(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        if current_call_frame.gas_used + gas_cost::GASPRICE > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        current_call_frame.stack.push(self.env.gas_price)?;

        self.increase_gas(current_call_frame, gas_cost::GASPRICE);

        Ok(OpcodeSuccess::Continue)
    }

    // EXTCODESIZE operation
    pub fn op_extcodesize(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let address = word_to_address(current_call_frame.stack.pop()?);
        let gas_cost = if self.accrued_substate.warm_addresses.contains(&address) {
            call_opcode::WARM_ADDRESS_ACCESS_COST
        } else {
            call_opcode::COLD_ADDRESS_ACCESS_COST
        };
        if current_call_frame.gas_used + gas_cost > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }
        let code_size = self.db.get_account_bytecode(&address).len();
        current_call_frame.stack.push(code_size.into())?;

        self.increase_gas(current_call_frame, gas_cost);
        Ok(OpcodeSuccess::Continue)
    }

    // EXTCODECOPY operation
    pub fn op_extcodecopy(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let address = word_to_address(current_call_frame.stack.pop()?);
        let dest_offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;
        let offset: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;
        let size: usize = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;

        let minimum_word_size = (size + WORD_SIZE - 1) / WORD_SIZE;
        let memory_expansion_cost =
            current_call_frame.memory.expansion_cost(dest_offset + size) as u64;
        let address_access_cost = if self.accrued_substate.warm_addresses.contains(&address) {
            call_opcode::WARM_ADDRESS_ACCESS_COST
        } else {
            call_opcode::COLD_ADDRESS_ACCESS_COST
        };
        let gas_cost = gas_cost::EXTCODECOPY_DYNAMIC_BASE * minimum_word_size as u64
            + memory_expansion_cost
            + address_access_cost;
        if current_call_frame.gas_used + gas_cost > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let mut code = self.db.get_account_bytecode(&address);
        if code.len() < offset + size {
            let mut extended_code = code.to_vec();
            extended_code.resize(offset + size, 0);
            code = Bytes::from(extended_code);
        }
        current_call_frame
            .memory
            .store_bytes(dest_offset, &code[offset..offset + size]);

        self.increase_gas(current_call_frame, gas_cost);
        Ok(OpcodeSuccess::Continue)
    }

    // EXTCODEHASH operation
    pub fn op_extcodehash(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let address = word_to_address(current_call_frame.stack.pop()?);
        let gas_cost = if self.accrued_substate.warm_addresses.contains(&address) {
            call_opcode::WARM_ADDRESS_ACCESS_COST
        } else {
            call_opcode::COLD_ADDRESS_ACCESS_COST
        };
        if current_call_frame.gas_used + gas_cost > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }

        let code = self.db.get_account_bytecode(&address);
        let mut hasher = Keccak256::new();
        hasher.update(code);
        let result = hasher.finalize();
        current_call_frame
            .stack
            .push(U256::from_big_endian(&result))?;

        self.increase_gas(current_call_frame, gas_cost);
        Ok(OpcodeSuccess::Continue)
    }
}

use std::str::FromStr;
fn address_to_word(address: Address) -> U256 {
    // This unwrap can't panic, as Address are 20 bytes long and U256 use 32 bytes
    U256::from_str(&format!("{address:?}")).unwrap()
}
