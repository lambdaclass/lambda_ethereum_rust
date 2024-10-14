use sha3::digest::consts::U2;

use crate::{
    constants::{call_opcode, SUCCESS_FOR_RETURN}, vm::Account, vm_result::ResultReason
};

use super::*;

// System Operations (10)
// Opcodes: CREATE, CALL, CALLCODE, RETURN, DELEGATECALL, CREATE2, STATICCALL, REVERT, INVALID, SELFDESTRUCT

impl VM {
    // CALL operation
    pub fn op_call(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let gas = current_call_frame.stack.pop()?;
        let code_address = Address::from_low_u64_be(current_call_frame.stack.pop()?.low_u64());
        let value = current_call_frame.stack.pop()?;
        let args_offset = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);
        let args_size = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);
        let ret_offset = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);
        let ret_size = current_call_frame
            .stack
            .pop()?
            .try_into()
            .unwrap_or(usize::MAX);

        let memory_byte_size = (args_offset + args_size).max(ret_offset + ret_size);
        let memory_expansion_cost = current_call_frame.memory.expansion_cost(memory_byte_size);

        let address_access_cost = if self.accrued_substate.warm_addresses.contains(&code_address) {
            call_opcode::WARM_ADDRESS_ACCESS_COST
        } else {
            call_opcode::COLD_ADDRESS_ACCESS_COST
        };

        let positive_value_cost = if !value.is_zero() {
            call_opcode::NON_ZERO_VALUE_COST + call_opcode::BASIC_FALLBACK_FUNCTION_STIPEND
        } else {
            0
        };

        let account = self.db.accounts.get(&code_address).unwrap(); // if the account doesn't exist, it should be created
        let value_to_empty_account_cost = if !value.is_zero() && account.is_empty() {
            call_opcode::VALUE_TO_EMPTY_ACCOUNT_COST
        } else {
            0
        };

        let gas_cost = memory_expansion_cost as u64
            + address_access_cost
            + positive_value_cost
            + value_to_empty_account_cost;

        if self.env.consumed_gas + gas_cost > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        self.env.consumed_gas += gas_cost;
        self.accrued_substate.warm_addresses.insert(code_address);

        let msg_sender = current_call_frame.msg_sender;
        let to = current_call_frame.to;
        let is_static = current_call_frame.is_static;

        self.generic_call(
            current_call_frame,
            gas,
            value,
            msg_sender,
            to,
            code_address,
            None,
            false,
            is_static,
            args_offset,
            args_size,
            ret_offset,
            ret_size,
        )?;

        Ok(OpcodeSuccess::Continue)
    }

    // CALLCODE operation
    pub fn op_callcode(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let gas = current_call_frame.stack.pop()?;
        let code_address = Address::from_low_u64_be(current_call_frame.stack.pop()?.low_u64());
        let value = current_call_frame.stack.pop()?;
        let args_offset = current_call_frame.stack.pop()?.try_into().unwrap();
        let args_size = current_call_frame.stack.pop()?.try_into().unwrap();
        let ret_offset = current_call_frame.stack.pop()?.try_into().unwrap();
        let ret_size = current_call_frame.stack.pop()?.try_into().unwrap();

        let msg_sender = current_call_frame.msg_sender;
        let to = current_call_frame.to;
        let is_static = current_call_frame.is_static;

        self.generic_call(
            current_call_frame,
            gas,
            value,
            code_address,
            to,
            code_address,
            Some(msg_sender),
            false,
            is_static,
            args_offset,
            args_size,
            ret_offset,
            ret_size,
        )?;

        Ok(OpcodeSuccess::Continue)
    }

    // RETURN operation
    pub fn op_return(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
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

        let gas_cost = current_call_frame.memory.expansion_cost(offset + size) as u64;
        if self.env.consumed_gas + gas_cost > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }

        self.env.consumed_gas += gas_cost;
        let return_data = current_call_frame.memory.load_range(offset, size).into();
        current_call_frame.returndata = return_data;
        current_call_frame
            .stack
            .push(U256::from(SUCCESS_FOR_RETURN))?;

        Ok(OpcodeSuccess::Result(ResultReason::Return))
    }

    // DELEGATECALL operation
    pub fn op_delegatecall(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let gas = current_call_frame.stack.pop()?;
        let code_address = Address::from_low_u64_be(current_call_frame.stack.pop()?.low_u64());
        let args_offset = current_call_frame.stack.pop()?.try_into().unwrap();
        let args_size = current_call_frame.stack.pop()?.try_into().unwrap();
        let ret_offset = current_call_frame.stack.pop()?.try_into().unwrap();
        let ret_size = current_call_frame.stack.pop()?.try_into().unwrap();

        let value = current_call_frame.msg_value;
        let msg_sender = current_call_frame.msg_sender;
        let to = current_call_frame.to;
        let is_static = current_call_frame.is_static;

        self.generic_call(
            current_call_frame,
            gas,
            value,
            msg_sender,
            to,
            code_address,
            Some(msg_sender),
            false,
            is_static,
            args_offset,
            args_size,
            ret_offset,
            ret_size,
        )?;

        Ok(OpcodeSuccess::Continue)
    }

    // STATICCALL operation
    pub fn op_staticcall(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let gas = current_call_frame.stack.pop()?;
        let code_address = Address::from_low_u64_be(current_call_frame.stack.pop()?.low_u64());
        let args_offset = current_call_frame.stack.pop()?.try_into().unwrap();
        let args_size = current_call_frame.stack.pop()?.try_into().unwrap();
        let ret_offset = current_call_frame.stack.pop()?.try_into().unwrap();
        let ret_size = current_call_frame.stack.pop()?.try_into().unwrap();

        let msg_sender = current_call_frame.msg_sender;
        let value = current_call_frame.msg_value;

        self.generic_call(
            current_call_frame,
            gas,
            value,
            msg_sender,
            code_address,
            code_address,
            None,
            false,
            true,
            args_offset,
            args_size,
            ret_offset,
            ret_size,
        )?;

        Ok(OpcodeSuccess::Continue)
    }

    // CREATE operation
    pub fn op_create(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let value_in_wei_to_send = current_call_frame.stack.pop()?;
        let code_offset_in_memory = current_call_frame.stack.pop()?.try_into().unwrap();
        let code_size_in_memory = current_call_frame.stack.pop()?.try_into().unwrap();

        self.create(
            value_in_wei_to_send,
            code_offset_in_memory,
            code_size_in_memory,
            None,
            current_call_frame,
        )?;

        Ok(OpcodeSuccess::Continue)
    }

    // CREATE2 operation
    pub fn op_create2(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        let value_in_wei_to_send = current_call_frame.stack.pop()?;
        let code_offset_in_memory = current_call_frame.stack.pop()?.try_into().unwrap();
        let code_size_in_memory = current_call_frame.stack.pop()?.try_into().unwrap();
        let salt = current_call_frame.stack.pop()?;

        self.create(
            value_in_wei_to_send,
            code_offset_in_memory,
            code_size_in_memory,
            Some(salt),
            current_call_frame,
        )?;

        Ok(OpcodeSuccess::Continue)
    }

    // REVERT operation
    pub fn op_revert(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        // Gets values from stack, calculates gas cost and calls revert function.

        let offset = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;

        let size = current_call_frame
            .stack
            .pop()?
            .try_into()
            .map_err(|_| VMError::VeryLargeNumber)?;

        let gas_cost = current_call_frame.memory.expansion_cost(offset + size) as u64;

        if self.env.consumed_gas + gas_cost > self.env.gas_limit { // This should be changed to callframe gas limit, not env limit.
            return Err(VMError::OutOfGas);
        }

        // Increment the consumed gas by the gas cost
        self.env.consumed_gas += gas_cost;

        current_call_frame.returndata = current_call_frame.memory.load_range(offset, size).into();

        Ok(OpcodeSuccess::Result(ResultReason::Revert))
    }

    /// ### INVALID operation
    /// Reverts consuming all gas, no return data.
    pub fn op_invalid(
        &mut self
    ) -> Result<OpcodeSuccess, VMError> {
        Err(VMError::InvalidOpcode)
    }

    // SELFDESTRUCT operation
    pub fn op_selfdestruct(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        // Sends all ether in the account to the target address
        // Steps:
        // 1. Pop the target address from the stack
        // 1': Store in a variable the balance of the current account
        // 2. Get the target account, checking if it is empty and if it is cold. Update gas cost accordingly.
        // 3. Check if gas cost exceeds gas limit (return OutOfGas if it does)
        // 4. Add the balance of the current account to the target account
        // 5. Set the balance of the current account to 0

        // Problems/Doubts: 
        //      Should I update db directly with get_mut? 
        //      How do I know if address is cold or warm? Is it in accesslist?
        //      How do I know if address is empty? It should have a balance of 0, nonce 0 and no code. But if an Account is not in the database, does it mean it is empty? I don't know what does our database represent, should I assume that all existing accounts are in there?

        // Notes: If context is Static, return error.
        if current_call_frame.is_static {
            return Err(VMError::OpcodeNotAllowedInStaticContext);
        }

        // Gas costs variables
        let static_gas_cost = gas_cost::SELFDESTRUCT_STATIC as u64;
        let dynamic_gas_cost = gas_cost::SELFDESTRUCT_DYNAMIC_BASE as u64;
        let cold_gas_cost = gas_cost::SELFDESTRUCT_DYNAMIC_COLD as u64;
        let mut gas_cost = static_gas_cost ; // This will be updated later

        
        // 1
        let target_address = Address::from_low_u64_be(current_call_frame.stack.pop()?.low_u64());

        // 1'
        let current_account_balance = self.db.accounts.get(&current_call_frame.to).unwrap().balance;

        
        // Get the target account (I HAVE TO CHECK IF ADDRESS IS COLD AND IF IT IS EMPTY AND UPDATE GAS COST ACCORDINGLY)
        // If a positive balance is sent to an empty account, the dynamic gas is 25000.
        // Additionaly, if address is cold, there is an additional cost of 2600.
        let target_account = match self.db.accounts.get_mut(&target_address) {
            Some(acc) => acc,
            None => {
                // If it is not in the database, does it mean it is an empty account?
                self.db.accounts.insert(target_address, Account::default());
                self.db.accounts.get_mut(&target_address).unwrap()
            }
        };
        
        if self.env.consumed_gas + gas_cost > self.env.gas_limit {
            return Err(VMError::OutOfGas);
        }
        
        
        target_account.balance += current_account_balance;
        
        
        
        // // Reset balance of the current account. (Last step)
        // let current_account = self.db.accounts.get_mut(&current_call_frame.to).unwrap();
        // current_account.balance = U256::zero();
        
        unimplemented!("SELFDESTRUCT opcode encountered")        
    }
}
