// bitwise_comparison.rs

use crate::{call_frame::CallFrame, vm::VM, vm_result::VMError};
use ethereum_types::U256;

impl VM {
    // AND operation
    pub fn and(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let a = current_call_frame.stack.pop()?;
        let b = current_call_frame.stack.pop()?;
        current_call_frame.stack.push(a & b)?;
        Ok(())
    }

    // OR operation
    pub fn or(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let a = current_call_frame.stack.pop()?;
        let b = current_call_frame.stack.pop()?;
        current_call_frame.stack.push(a | b)?;
        Ok(())
    }

    // XOR operation
    pub fn xor(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let a = current_call_frame.stack.pop()?;
        let b = current_call_frame.stack.pop()?;
        current_call_frame.stack.push(a ^ b)?;
        Ok(())
    }

    // NOT operation
    pub fn not(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let a = current_call_frame.stack.pop()?;
        current_call_frame.stack.push(!a)?;
        Ok(())
    }

    // BYTE operation
    pub fn byte(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let op1 = current_call_frame.stack.pop()?;
        let op2 = current_call_frame.stack.pop()?;

        let byte_index = op1.try_into().unwrap_or(usize::MAX);

        if byte_index < 32 {
            current_call_frame.stack.push(U256::from(op2.byte(31 - byte_index)))?;
        } else {
            current_call_frame.stack.push(U256::zero())?;
        }
        Ok(())
    }

    // SHL operation (Shift Left)
    pub fn shl(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let shift = current_call_frame.stack.pop()?;
        let value = current_call_frame.stack.pop()?;
        if shift < U256::from(256) {
            current_call_frame.stack.push(value << shift)?;
        } else {
            current_call_frame.stack.push(U256::zero())?;
        }
        Ok(())
    }

    // SHR operation (Shift Right)
    pub fn shr(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let shift = current_call_frame.stack.pop()?;
        let value = current_call_frame.stack.pop()?;
        if shift < U256::from(256) {
            current_call_frame.stack.push(value >> shift)?;
        } else {
            current_call_frame.stack.push(U256::zero())?;
        }
        Ok(())
    }

    // SAR operation (Arithmetic Shift Right)
    pub fn sar(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let shift = current_call_frame.stack.pop()?;
        let value = current_call_frame.stack.pop()?;
        let res = if shift < U256::from(256) {
            arithmetic_shift_right(value, shift)
        } else if value.bit(255) {
            U256::MAX
        } else {
            U256::zero()
        };
        current_call_frame.stack.push(res)?;
        Ok(())
    }
}

fn arithmetic_shift_right(value: U256, shift: U256) -> U256 {
    let shift_usize: usize = shift.try_into().unwrap(); // we know its not bigger than 256

    if value.bit(255) {
        // if negative fill with 1s
        let shifted = value >> shift_usize;
        let mask = U256::MAX << (256 - shift_usize);
        shifted | mask
    } else {
        value >> shift_usize
    }
}
