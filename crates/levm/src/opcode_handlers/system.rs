use crate::{constants::{HALT_FOR_CALL, REVERT_FOR_CALL, SUCCESS_FOR_CALL, SUCCESS_FOR_RETURN}, vm_result::{ExecutionResult, ResultReason}};

use super::*;

// System Operations (10)
// Opcodes: CREATE, CALL, CALLCODE, RETURN, DELEGATECALL, CREATE2, STATICCALL, REVERT, INVALID, SELFDESTRUCT

impl VM {
    pub fn op_create(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_call(&mut self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let gas = current_call_frame.stack.pop()?;
        let address = Address::from_low_u64_be(current_call_frame.stack.pop()?.low_u64());
        let value = current_call_frame.stack.pop()?;
        let args_offset = current_call_frame.stack.pop()?.try_into().unwrap_or(usize::MAX);
        let args_size = current_call_frame.stack.pop()?.try_into().unwrap_or(usize::MAX);
        let ret_offset = current_call_frame.stack.pop()?.try_into().unwrap_or(usize::MAX);
        let ret_size = current_call_frame.stack.pop()?.try_into().unwrap_or(usize::MAX);

        // check balance
        if self.balance(&current_call_frame.msg_sender) < value {
            current_call_frame.stack.push(U256::from(REVERT_FOR_CALL))?;
            return Ok(());
        }

        // transfer value
        // transfer(&current_call_frame.msg_sender, &address, value);
        let callee_bytecode = self.get_account_bytecode(&address);
        if callee_bytecode.is_empty() {
            current_call_frame.stack.push(U256::from(SUCCESS_FOR_CALL))?;
            return Ok(());
        }

        let calldata = current_call_frame.memory.load_range(args_offset, args_size).into();

        let new_call_frame = CallFrame {
            gas,
            msg_sender: current_call_frame.msg_sender, // caller remains the msg_sender
            callee: address,
            bytecode: callee_bytecode,
            msg_value: value,
            calldata,
            ..Default::default()
        };

        current_call_frame.return_data_offset = Some(ret_offset);
        current_call_frame.return_data_size = Some(ret_size);
        self.call_frames.push(new_call_frame.clone());
        let result = self.execute();

        match result {
            Ok(ExecutionResult::Success { logs, return_data, .. }) => {
                current_call_frame.logs.extend(logs);
                current_call_frame.memory.store_bytes(ret_offset, &return_data);
                current_call_frame.returndata = return_data;
                current_call_frame.stack.push(U256::from(SUCCESS_FOR_CALL))?;
            }
            Err(_) => {
                current_call_frame.stack.push(U256::from(HALT_FOR_CALL))?;
            }
        };

        Ok(())
    }

    pub fn op_callcode(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_return(&self, current_call_frame: &mut CallFrame) -> Result<ExecutionResult, VMError> {
        let offset = current_call_frame.stack.pop()?.try_into().unwrap_or(usize::MAX);
        let size = current_call_frame.stack.pop()?.try_into().unwrap_or(usize::MAX);
        let return_data = current_call_frame.memory.load_range(offset, size).into();

        current_call_frame.returndata = return_data;
        current_call_frame.stack.push(U256::from(SUCCESS_FOR_RETURN))?;
        return Ok(Self::write_success_result(current_call_frame.clone(), ResultReason::Return));
    }

    pub fn op_delegatecall(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_create2(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_staticcall(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_revert(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_invalid(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    pub fn op_selfdestruct(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        Ok(())
    }

    fn get_account_bytecode(&mut self, address: &Address) -> Bytes {
        self.accounts
            .get(address)
            .map_or(Bytes::new(), |acc| acc.bytecode.clone())
    }

    fn balance(&mut self, address: &Address) -> U256 {
        self.accounts
            .get(address)
            .map_or(U256::zero(), |acc| acc.balance)
    }
}
