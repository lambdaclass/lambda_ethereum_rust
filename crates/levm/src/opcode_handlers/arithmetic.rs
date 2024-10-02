use crate::{call_frame::CallFrame, vm::VM, vm_result::VMError};
use ethereum_types::{U256, U512};

use super::{is_negative, negate};

impl VM {
    // ADD operation
    pub fn add(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let augend = current_call_frame.stack.pop()?;
        let addend = current_call_frame.stack.pop()?;
        let sum = augend.overflowing_add(addend).0;
        current_call_frame.stack.push(sum)?;
        Ok(())
    }

    // SUB operation
    pub fn sub(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let minuend = current_call_frame.stack.pop()?;
        let subtrahend = current_call_frame.stack.pop()?;
        let difference = minuend.overflowing_sub(subtrahend).0;
        current_call_frame.stack.push(difference)?;
        Ok(())
    }

    // MUL operation
    pub fn mul(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let multiplicand = current_call_frame.stack.pop()?;
        let multiplier = current_call_frame.stack.pop()?;
        let product = multiplicand.overflowing_mul(multiplier).0;
        current_call_frame.stack.push(product)?;
        Ok(())
    }

    // DIV operation
    pub fn div(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let dividend = current_call_frame.stack.pop()?;
        let divisor = current_call_frame.stack.pop()?;
        let quotient = if divisor.is_zero() {
            U256::zero()
        } else {
            dividend / divisor
        };
        current_call_frame.stack.push(quotient)?;
        Ok(())
    }

    // SDIV operation
    pub fn sdiv(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let dividend = current_call_frame.stack.pop()?;
        let divisor = current_call_frame.stack.pop()?;
        if divisor.is_zero() {
            current_call_frame.stack.push(U256::zero())?;
        } else {
            let dividend_is_negative = is_negative(dividend);
            let divisor_is_negative = is_negative(divisor);

            let dividend = if dividend_is_negative {
                negate(dividend)
            } else {
                dividend
            };
            let divisor = if divisor_is_negative {
                negate(divisor)
            } else {
                divisor
            };

            let quotient = dividend / divisor;
            let quotient_is_negative = dividend_is_negative ^ divisor_is_negative;

            let quotient = if quotient_is_negative {
                negate(quotient)
            } else {
                quotient
            };

            current_call_frame.stack.push(quotient)?;
        }
        Ok(())
    }

    // MOD operation
    pub fn modulus(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let dividend = current_call_frame.stack.pop()?;
        let divisor = current_call_frame.stack.pop()?;
        let remainder = if divisor.is_zero() {
            U256::zero()
        } else {
            dividend % divisor
        };
        current_call_frame.stack.push(remainder)?;
        Ok(())
    }

    // SMOD operation
    pub fn smod(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let dividend = current_call_frame.stack.pop()?;
        let divisor = current_call_frame.stack.pop()?;
        if divisor.is_zero() {
            current_call_frame.stack.push(U256::zero())?;
        } else {
            let dividend_is_negative = is_negative(dividend);
            let divisor_is_negative = is_negative(divisor);

            let dividend = if dividend_is_negative {
                negate(dividend)
            } else {
                dividend
            };
            let divisor = if divisor_is_negative {
                negate(divisor)
            } else {
                divisor
            };

            let remainder = dividend % divisor;
            let remainder_is_negative = dividend_is_negative ^ divisor_is_negative;

            let remainder = if remainder_is_negative {
                negate(remainder)
            } else {
                remainder
            };

            current_call_frame.stack.push(remainder)?;
        }
        Ok(())
    }

    // ADDMOD operation
    pub fn addmod(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let augend = current_call_frame.stack.pop()?;
        let addend = current_call_frame.stack.pop()?;
        let divisor = current_call_frame.stack.pop()?;
        if divisor.is_zero() {
            current_call_frame.stack.push(U256::zero())?;
            return Ok(());
        }
        let (sum, overflow) = augend.overflowing_add(addend);
        let mut remainder = sum % divisor;
        if overflow || remainder > divisor {
            remainder = remainder.overflowing_sub(divisor).0;
        }

        current_call_frame.stack.push(remainder)?;
        Ok(())
    }

    // MULMOD operation
    pub fn mulmod(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let multiplicand = U512::from(current_call_frame.stack.pop()?);

        let multiplier = U512::from(current_call_frame.stack.pop()?);
        let divisor = U512::from(current_call_frame.stack.pop()?);
        if divisor.is_zero() {
            current_call_frame.stack.push(U256::zero())?;
            return Ok(());
        }

        let (product, overflow) = multiplicand.overflowing_mul(multiplier);
        let mut remainder = product % divisor;
        if overflow || remainder > divisor {
            remainder = remainder.overflowing_sub(divisor).0;
        }
        let mut result = Vec::new();
        for byte in remainder.0.iter().take(4) {
            let bytes = byte.to_le_bytes();
            result.extend_from_slice(&bytes);
        }
        // before reverse we have something like [120, 255, 0, 0....]
        // after reverse we get the [0, 0, ...., 255, 120] which is the correct order for the little endian u256
        result.reverse();
        let remainder = U256::from(result.as_slice());
        current_call_frame.stack.push(remainder)?;
        Ok(())
    }

    // EXP operation
    pub fn exp(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let base = current_call_frame.stack.pop()?;
        let exponent = current_call_frame.stack.pop()?;
        let power = base.overflowing_pow(exponent).0;
        current_call_frame.stack.push(power)?;
        Ok(())
    }

    // SIGNEXTEND operation
    pub fn signextend(current_call_frame: &mut CallFrame) -> Result<(), VMError> {
        let byte_size = current_call_frame.stack.pop()?;
        let value_to_extend = current_call_frame.stack.pop()?;

        let bits_per_byte = U256::from(8);
        let sign_bit_position_on_byte = 7;
        let max_byte_size = 31;

        let byte_size = byte_size.min(U256::from(max_byte_size));
        let sign_bit_index = bits_per_byte * byte_size + sign_bit_position_on_byte;
        let is_negative = value_to_extend.bit(sign_bit_index.as_usize());
        let sign_bit_mask = (U256::one() << sign_bit_index) - U256::one();
        let result = if is_negative {
            value_to_extend | !sign_bit_mask
        } else {
            value_to_extend & sign_bit_mask
        };
        current_call_frame.stack.push(result)?;
        Ok(())
    }
}
