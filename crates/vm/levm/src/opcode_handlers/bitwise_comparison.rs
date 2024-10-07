// Comparison and Bitwise Logic Operations (14)
// Opcodes: LT, GT, SLT, SGT, EQ, ISZERO, AND, OR, XOR, NOT, BYTE, SHL, SHR, SAR
use super::*;

impl VM {
    // AND operation
    pub fn op_and(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let a = current_call_frame.stack.pop()?;
        let b = current_call_frame.stack.pop()?;
        current_call_frame.stack.push(a & b)?;
        Ok(())
    }

    // OR operation
    pub fn op_or(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let a = current_call_frame.stack.pop()?;
        let b = current_call_frame.stack.pop()?;
        current_call_frame.stack.push(a | b)?;
        Ok(())
    }

    // XOR operation
    pub fn op_xor(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let a = current_call_frame.stack.pop()?;
        let b = current_call_frame.stack.pop()?;
        current_call_frame.stack.push(a ^ b)?;
        Ok(())
    }

    // NOT operation
    pub fn op_not(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let a = current_call_frame.stack.pop()?;
        current_call_frame.stack.push(!a)?;
        Ok(())
    }

    // BYTE operation
    pub fn op_byte(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let op1 = current_call_frame.stack.pop()?;
        let op2 = current_call_frame.stack.pop()?;

        let byte_index = op1.try_into().unwrap_or(usize::MAX);

        if byte_index < 32 {
            current_call_frame
                .stack
                .push(U256::from(op2.byte(31 - byte_index)))?;
        } else {
            current_call_frame.stack.push(U256::zero())?;
        }
        Ok(())
    }

    // SHL operation (Shift Left)
    pub fn op_shl(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
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
    pub fn op_shr(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
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
    pub fn op_sar(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
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

    // LT operation (Less Than)
    pub fn op_lt(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let lho = current_call_frame.stack.pop()?;
        let rho = current_call_frame.stack.pop()?;
        let result = if lho < rho { U256::one() } else { U256::zero() };
        current_call_frame.stack.push(result)?;
        Ok(())
    }

    // GT operation (Greater Than)
    pub fn op_gt(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let lho = current_call_frame.stack.pop()?;
        let rho = current_call_frame.stack.pop()?;
        let result = if lho > rho { U256::one() } else { U256::zero() };
        current_call_frame.stack.push(result)?;
        Ok(())
    }

    // SLT operation (Signed Less Than)
    pub fn op_slt(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let lho = current_call_frame.stack.pop()?;
        let rho = current_call_frame.stack.pop()?;
        let lho_is_negative = lho.bit(255);
        let rho_is_negative = rho.bit(255);
        let result = if lho_is_negative == rho_is_negative {
            // if both have the same sign, compare their magnitudes
            if lho < rho {
                U256::one()
            } else {
                U256::zero()
            }
        } else {
            // if they have different signs, the negative number is smaller
            if lho_is_negative {
                U256::one()
            } else {
                U256::zero()
            }
        };
        current_call_frame.stack.push(result)?;
        Ok(())
    }

    // SGT operation (Signed Greater Than)
    pub fn op_sgt(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let lho = current_call_frame.stack.pop()?;
        let rho = current_call_frame.stack.pop()?;
        let lho_is_negative = lho.bit(255);
        let rho_is_negative = rho.bit(255);
        let result = if lho_is_negative == rho_is_negative {
            // if both have the same sign, compare their magnitudes
            if lho > rho {
                U256::one()
            } else {
                U256::zero()
            }
        } else {
            // if they have different signs, the positive number is bigger
            if rho_is_negative {
                U256::one()
            } else {
                U256::zero()
            }
        };
        current_call_frame.stack.push(result)?;
        Ok(())
    }

    // EQ operation (Equal)
    pub fn op_eq(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let lho = current_call_frame.stack.pop()?;
        let rho = current_call_frame.stack.pop()?;
        let result = if lho == rho {
            U256::one()
        } else {
            U256::zero()
        };
        current_call_frame.stack.push(result)?;
        Ok(())
    }

    // ISZERO operation
    pub fn op_iszero(&self, current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let operand = current_call_frame.stack.pop()?;
        let result = if operand == U256::zero() {
            U256::one()
        } else {
            U256::zero()
        };
        current_call_frame.stack.push(result)?;
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
