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

        

        self.increase_consumed_gas(current_call_frame, gas_cost)?;
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
        )
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
        )
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
        
        self.increase_consumed_gas(current_call_frame, gas_cost)?;

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
        )
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
        )
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
        )
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
        )
    }

    // REVERT operation
    pub fn op_revert(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        // Description: Gets values from stack, calculates gas cost and sets return data.
        // Returns: Revert as Result Reason. VMError otherwise. 
        // Notes:
        //      The reversion of changes is made in the generic_call(). 
        //      Changes are not "reverted" if it is the first callframe, they are just not commited.

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

        self.increase_consumed_gas(current_call_frame, gas_cost)?;
        
        current_call_frame.returndata = current_call_frame.memory.load_range(offset, size).into();

        Err(VMError::RevertOpcode)
    }

    /// ### INVALID operation
    /// Reverts consuming all gas, no return data.
    pub fn op_invalid(
        &mut self
    ) -> Result<OpcodeSuccess, VMError> {
        Err(VMError::InvalidOpcode)
    }


    // selfdestruct(address). Agarra todo el ether de un contrato y se lo da a un address.
    // SELFDESTRUCT operation
    pub fn op_selfdestruct(
        &mut self,
        current_call_frame: &mut CallFrame,
    ) -> Result<OpcodeSuccess, VMError> {
        // Sends all ether in the account to the target address
        // Steps:
        // 1. Pop the target address from the stack
        // 2. Get current account and: Store the balance in a variable, set it's balance to 0
        // 3. Get the target account, checking if it is empty and if it is cold. Update gas cost accordingly.
        // 4. Add the balance of the current account to the target account
        // 5. Register account to be destroyed in accrued substate.

        // Notes: 
        //      If context is Static, return error.
        //      If executed in the same transaction a contract was created, the current account is registered to be destroyed
        if current_call_frame.is_static {
            return Err(VMError::OpcodeNotAllowedInStaticContext);
        }

        // Gas costs variables
        let static_gas_cost = gas_cost::SELFDESTRUCT_STATIC;
        let dynamic_gas_cost = gas_cost::SELFDESTRUCT_DYNAMIC_BASE;
        let cold_gas_cost = gas_cost::SELFDESTRUCT_DYNAMIC_COLD;
        let mut gas_cost = static_gas_cost; // This will be updated later

        
        // 1. Pop the target address from the stack
        let target_address = Address::from_low_u64_be(current_call_frame.stack.pop()?.low_u64());

        // 2. Get current account and: Store the balance in a variable, set it's balance to 0
        let current_account_balance = self.db.accounts.get(&current_call_frame.to).unwrap().balance;
        self.db.accounts.get_mut(&current_call_frame.to).unwrap().balance = U256::zero();

        
        // 3. Get the target account, checking if it is empty and if it is cold. Update gas cost accordingly.

        // If address is cold, there is an additional cost of 2600. AFAIK accessList has not been implemented yet.

        // If a positive balance is sent to an empty account, the dynamic gas is 25000.
        let target_account = match self.db.accounts.get_mut(&target_address) {
            Some(acc) => acc,
            None => {
                // I'm considering that if address is not in the database, it means that it is an empty account.
                gas_cost += dynamic_gas_cost;
                self.db.accounts.insert(target_address, Account::default());
                self.db.accounts.get_mut(&target_address).unwrap()
            }
        };
        
        // 4. Add the balance of the current account to the target account
        target_account.balance += current_account_balance;
        
        // 5. Register account to be destroyed in accrued substate IF executed in the same transaction a contract was created
        if self.accrued_substate.created_contracts.contains(&current_call_frame.to) {
           self.accrued_substate.self_destruct_set.insert(current_call_frame.to);
        }
        // Those accounts should be destroyed at the end of the transaction.
        
        self.increase_consumed_gas(current_call_frame, gas_cost)?;

        Ok(OpcodeSuccess::Result(ResultReason::SelfDestruct))    
    }
}
