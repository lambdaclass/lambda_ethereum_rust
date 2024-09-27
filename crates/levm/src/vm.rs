use crate::{
    call_frame::CallFrame,
    constants::{gas_cost, TX_BASE_COST, WORD_SIZE},
    opcodes::Opcode,
};
use bytes::Bytes;
use ethereum_types::{U256, U512};
use sha3::{Digest, Keccak256};
use std::i64;

#[derive(Debug, Clone, Default)]
pub struct VM {
    pub call_frames: Vec<CallFrame>,
}

/// Shifts the value to the right by 255 bits and checks the most significant bit is a 1
fn is_negative(value: U256) -> bool {
    value.bit(255)
}
/// negates a number in two's complement
fn negate(value: U256) -> U256 {
    !value + U256::one()
}

impl VM {
    pub fn new(bytecode: Bytes) -> Self {
        let initial_call_frame = CallFrame {
            bytecode,
            ..Default::default()
        };
        Self {
            call_frames: vec![initial_call_frame],
        }
    }

    pub fn execute(&mut self) {
        let gas_limit = i64::MAX as u64; // TODO: it was initialized like this on evm_mlir, check why
        let mut consumed_gas = TX_BASE_COST; // TODO: check where to place these two, probably TxEnv
        let current_call_frame = self.current_call_frame();
        loop {
            match current_call_frame.next_opcode().unwrap() {
                Opcode::STOP => break,
                Opcode::ADD => {
                    if consumed_gas + gas_cost::ADD > gas_limit {
                        break; // should revert the tx
                    }
                    let augend = current_call_frame.stack.pop().unwrap();
                    let addend = current_call_frame.stack.pop().unwrap();
                    let sum = augend.overflowing_add(addend).0;
                    current_call_frame.stack.push(sum);
                    consumed_gas += gas_cost::ADD
                }
                Opcode::MUL => {
                    if consumed_gas + gas_cost::MUL > gas_limit {
                        break; // should revert the tx
                    }
                    let multiplicand = current_call_frame.stack.pop().unwrap();
                    let multiplier = current_call_frame.stack.pop().unwrap();
                    let product = multiplicand.overflowing_mul(multiplier).0;
                    current_call_frame.stack.push(product);
                    consumed_gas += gas_cost::MUL
                }
                Opcode::SUB => {
                    if consumed_gas + gas_cost::SUB > gas_limit {
                        break; // should revert the tx
                    }
                    let minuend = current_call_frame.stack.pop().unwrap();
                    let subtrahend = current_call_frame.stack.pop().unwrap();
                    let difference = minuend.overflowing_sub(subtrahend).0;
                    current_call_frame.stack.push(difference);
                    consumed_gas += gas_cost::SUB
                }
                Opcode::DIV => {
                    if consumed_gas + gas_cost::DIV > gas_limit {
                        break; // should revert the tx
                    }
                    let dividend = current_call_frame.stack.pop().unwrap();
                    let divisor = current_call_frame.stack.pop().unwrap();
                    if divisor.is_zero() {
                        current_call_frame.stack.push(U256::zero());
                        continue;
                    }
                    let quotient = dividend / divisor;
                    current_call_frame.stack.push(quotient);
                    consumed_gas += gas_cost::DIV
                }
                Opcode::SDIV => {
                    if consumed_gas + gas_cost::SDIV > gas_limit {
                        break; // should revert the tx
                    }
                    let dividend = current_call_frame.stack.pop().unwrap();
                    let divisor = current_call_frame.stack.pop().unwrap();
                    if divisor.is_zero() {
                        current_call_frame.stack.push(U256::zero());
                        continue;
                    }

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

                    current_call_frame.stack.push(quotient);
                    consumed_gas += gas_cost::SDIV
                }
                Opcode::MOD => {
                    if consumed_gas + gas_cost::MOD > gas_limit {
                        break; // should revert the tx
                    }
                    let dividend = current_call_frame.stack.pop().unwrap();
                    let divisor = current_call_frame.stack.pop().unwrap();
                    if divisor.is_zero() {
                        current_call_frame.stack.push(U256::zero());
                        continue;
                    }
                    let remainder = dividend % divisor;
                    current_call_frame.stack.push(remainder);
                    consumed_gas += gas_cost::MOD
                }
                Opcode::SMOD => {
                    if consumed_gas + gas_cost::SMOD > gas_limit {
                        break; // should revert the tx
                    }
                    let dividend = current_call_frame.stack.pop().unwrap();
                    let divisor = current_call_frame.stack.pop().unwrap();
                    if divisor.is_zero() {
                        current_call_frame.stack.push(U256::zero());
                        continue;
                    }

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

                    current_call_frame.stack.push(remainder);
                    consumed_gas += gas_cost::MOD
                }
                Opcode::ADDMOD => {
                    if consumed_gas + gas_cost::ADDMOD > gas_limit {
                        break; // should revert the tx
                    }
                    let augend = current_call_frame.stack.pop().unwrap();
                    let addend = current_call_frame.stack.pop().unwrap();
                    let divisor = current_call_frame.stack.pop().unwrap();
                    if divisor.is_zero() {
                        current_call_frame.stack.push(U256::zero());
                        continue;
                    }
                    let (sum, overflow) = augend.overflowing_add(addend);
                    let mut remainder = sum % divisor;
                    if overflow || remainder > divisor {
                        remainder = remainder.overflowing_sub(divisor).0;
                    }

                    current_call_frame.stack.push(remainder);
                    consumed_gas += gas_cost::ADDMOD
                }
                Opcode::MULMOD => {
                    if consumed_gas + gas_cost::MULMOD > gas_limit {
                        break; // should revert the tx
                    }
                    let multiplicand = U512::from(current_call_frame.stack.pop().unwrap());
                    let multiplier = U512::from(current_call_frame.stack.pop().unwrap());
                    let divisor = U512::from(current_call_frame.stack.pop().unwrap());
                    if divisor.is_zero() {
                        current_call_frame.stack.push(U256::zero());
                        continue;
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
                    current_call_frame.stack.push(remainder);
                    consumed_gas += gas_cost::MULMOD
                }
                Opcode::EXP => {
                    let base = current_call_frame.stack.pop().unwrap();
                    let exponent = current_call_frame.stack.pop().unwrap();

                    let exponent_byte_size = (exponent.bits() as u64 + 7) / 8;
                    let gas_cost =
                        gas_cost::EXP_STATIC + gas_cost::EXP_DYNAMIC_BASE * exponent_byte_size;
                    if consumed_gas + gas_cost > gas_limit {
                        break; // should revert the tx
                    }

                    let power = base.overflowing_pow(exponent).0;
                    current_call_frame.stack.push(power);
                    consumed_gas += gas_cost
                }
                Opcode::SIGNEXTEND => {
                    if consumed_gas + gas_cost::SIGNEXTEND > gas_limit {
                        break; // should revert the tx
                    }
                    let byte_size = current_call_frame.stack.pop().unwrap();
                    let value_to_extend = current_call_frame.stack.pop().unwrap();

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
                    current_call_frame.stack.push(result);
                    consumed_gas += gas_cost::SIGNEXTEND
                }
                Opcode::LT => {
                    if consumed_gas + gas_cost::LT > gas_limit {
                        break; // should revert the tx
                    }
                    let lho = current_call_frame.stack.pop().unwrap();
                    let rho = current_call_frame.stack.pop().unwrap();
                    let result = if lho < rho { U256::one() } else { U256::zero() };
                    current_call_frame.stack.push(result);
                    consumed_gas += gas_cost::LT
                }
                Opcode::GT => {
                    if consumed_gas + gas_cost::GT > gas_limit {
                        break; // should revert the tx
                    }
                    let lho = current_call_frame.stack.pop().unwrap();
                    let rho = current_call_frame.stack.pop().unwrap();
                    let result = if lho > rho { U256::one() } else { U256::zero() };
                    current_call_frame.stack.push(result);
                    consumed_gas += gas_cost::GT
                }
                Opcode::SLT => {
                    if consumed_gas + gas_cost::SLT > gas_limit {
                        break; // should revert the tx
                    }
                    let lho = current_call_frame.stack.pop().unwrap();
                    let rho = current_call_frame.stack.pop().unwrap();
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
                    current_call_frame.stack.push(result);
                    consumed_gas += gas_cost::SLT
                }
                Opcode::SGT => {
                    if consumed_gas + gas_cost::SGT > gas_limit {
                        break; // should revert the tx
                    }
                    let lho = current_call_frame.stack.pop().unwrap();
                    let rho = current_call_frame.stack.pop().unwrap();
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
                    current_call_frame.stack.push(result);
                    consumed_gas += gas_cost::SGT
                }
                Opcode::EQ => {
                    if consumed_gas + gas_cost::EQ > gas_limit {
                        break; // should revert the tx
                    }
                    let lho = current_call_frame.stack.pop().unwrap();
                    let rho = current_call_frame.stack.pop().unwrap();
                    let result = if lho == rho {
                        U256::one()
                    } else {
                        U256::zero()
                    };
                    current_call_frame.stack.push(result);
                    consumed_gas += gas_cost::EQ
                }
                Opcode::ISZERO => {
                    if consumed_gas + gas_cost::ISZERO > gas_limit {
                        break; // should revert the tx
                    }
                    let operand = current_call_frame.stack.pop().unwrap();
                    let result = if operand == U256::zero() {
                        U256::one()
                    } else {
                        U256::zero()
                    };
                    current_call_frame.stack.push(result);
                    consumed_gas += gas_cost::ISZERO
                }
                Opcode::KECCAK256 => {
                    let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let size = current_call_frame.stack.pop().unwrap().try_into().unwrap();

                    let minimum_word_size = (size + WORD_SIZE - 1) / WORD_SIZE;
                    let memory_expansion_cost =
                        current_call_frame.memory.expansion_cost(offset + size);
                    let gas_cost = gas_cost::KECCAK25_STATIC
                        + gas_cost::KECCAK25_DYNAMIC_BASE * minimum_word_size as u64
                        + memory_expansion_cost;
                    if consumed_gas + gas_cost > gas_limit {
                        break; // should revert the tx
                    }

                    let value_bytes = current_call_frame.memory.load_range(offset, size);

                    let mut hasher = Keccak256::new();
                    hasher.update(value_bytes);
                    let result = hasher.finalize();
                    current_call_frame
                        .stack
                        .push(U256::from_big_endian(&result));
                    consumed_gas += gas_cost
                }
                Opcode::JUMP => {
                    if consumed_gas + gas_cost::JUMP > gas_limit {
                        break; // should revert the tx
                    }
                    let jump_address = current_call_frame.stack.pop().unwrap();
                    current_call_frame.jump(jump_address);
                    consumed_gas += gas_cost::JUMP
                }
                Opcode::JUMPI => {
                    if consumed_gas + gas_cost::JUMPI > gas_limit {
                        break; // should revert the tx
                    }
                    let jump_address = current_call_frame.stack.pop().unwrap();
                    let condition = current_call_frame.stack.pop().unwrap();
                    if condition != U256::zero() {
                        current_call_frame.jump(jump_address);
                    }
                    consumed_gas += gas_cost::JUMPI
                }
                Opcode::JUMPDEST => {
                    // just consume some gas, jumptable written at the start
                    if consumed_gas + gas_cost::JUMPDEST > gas_limit {
                        break; // should revert the tx
                    }
                    consumed_gas += gas_cost::JUMPDEST
                }
                Opcode::PC => {
                    if consumed_gas + gas_cost::PC > gas_limit {
                        break; // should revert the tx
                    }
                    current_call_frame
                        .stack
                        .push(U256::from(current_call_frame.pc - 1));
                    consumed_gas += gas_cost::PC
                }
                Opcode::PUSH0 => {
                    if consumed_gas + gas_cost::PUSH0 > gas_limit {
                        break; // should revert the tx
                    }
                    current_call_frame.stack.push(U256::zero());
                    consumed_gas += gas_cost::PUSH0
                }
                // PUSHn
                op if (Opcode::PUSH1..Opcode::PUSH32).contains(&op) => {
                    if consumed_gas + gas_cost::PUSHN > gas_limit {
                        break; // should revert the tx
                    }
                    let n_bytes = (op as u8) - (Opcode::PUSH1 as u8) + 1;
                    let next_n_bytes = current_call_frame
                        .bytecode
                        .get(current_call_frame.pc()..current_call_frame.pc() + n_bytes as usize)
                        .expect("invalid bytecode");
                    let value_to_push = U256::from(next_n_bytes);
                    current_call_frame.stack.push(value_to_push);
                    current_call_frame.increment_pc_by(n_bytes as usize);
                    consumed_gas += gas_cost::PUSHN
                }
                Opcode::PUSH32 => {
                    if consumed_gas + gas_cost::PUSHN > gas_limit {
                        break; // should revert the tx
                    }
                    let next_32_bytes = current_call_frame
                        .bytecode
                        .get(current_call_frame.pc()..current_call_frame.pc() + WORD_SIZE)
                        .unwrap();
                    let value_to_push = U256::from(next_32_bytes);
                    current_call_frame.stack.push(value_to_push);
                    current_call_frame.increment_pc_by(WORD_SIZE);
                    consumed_gas += gas_cost::PUSHN
                }
                Opcode::AND => {
                    if consumed_gas + gas_cost::AND > gas_limit {
                        break; // should revert the tx
                    }
                    let a = current_call_frame.stack.pop().unwrap();
                    let b = current_call_frame.stack.pop().unwrap();
                    current_call_frame.stack.push(a & b);
                    consumed_gas += gas_cost::AND
                }
                Opcode::OR => {
                    if consumed_gas + gas_cost::OR > gas_limit {
                        break; // should revert the tx
                    }
                    let a = current_call_frame.stack.pop().unwrap();
                    let b = current_call_frame.stack.pop().unwrap();
                    current_call_frame.stack.push(a | b);
                    consumed_gas += gas_cost::OR
                }
                Opcode::XOR => {
                    if consumed_gas + gas_cost::XOR > gas_limit {
                        break; // should revert the tx
                    }
                    let a = current_call_frame.stack.pop().unwrap();
                    let b = current_call_frame.stack.pop().unwrap();
                    current_call_frame.stack.push(a ^ b);
                    consumed_gas += gas_cost::XOR
                }
                Opcode::NOT => {
                    if consumed_gas + gas_cost::NOT > gas_limit {
                        break; // should revert the tx
                    }
                    let a = current_call_frame.stack.pop().unwrap();
                    current_call_frame.stack.push(!a);
                    consumed_gas += gas_cost::NOT
                }
                Opcode::BYTE => {
                    if consumed_gas + gas_cost::BYTE > gas_limit {
                        break; // should revert the tx
                    }
                    let op1 = current_call_frame.stack.pop().unwrap();
                    let op2 = current_call_frame.stack.pop().unwrap();

                    let byte_index = op1.try_into().unwrap_or(usize::MAX);

                    if byte_index < WORD_SIZE {
                        current_call_frame
                            .stack
                            .push(U256::from(op2.byte(WORD_SIZE - 1 - byte_index)));
                    } else {
                        current_call_frame.stack.push(U256::zero());
                    }
                    consumed_gas += gas_cost::BYTE
                }
                Opcode::SHL => {
                    if consumed_gas + gas_cost::SHL > gas_limit {
                        break; // should revert the tx
                    }
                    let shift = current_call_frame.stack.pop().unwrap();
                    let value = current_call_frame.stack.pop().unwrap();
                    if shift < U256::from(256) {
                        current_call_frame.stack.push(value << shift);
                    } else {
                        current_call_frame.stack.push(U256::zero());
                    }
                    consumed_gas += gas_cost::SHL
                }
                Opcode::SHR => {
                    if consumed_gas + gas_cost::SHR > gas_limit {
                        break; // should revert the tx
                    }
                    let shift = current_call_frame.stack.pop().unwrap();
                    let value = current_call_frame.stack.pop().unwrap();
                    if shift < U256::from(256) {
                        current_call_frame.stack.push(value >> shift);
                    } else {
                        current_call_frame.stack.push(U256::zero());
                    }
                    consumed_gas += gas_cost::SHR
                }
                Opcode::SAR => {
                    if consumed_gas + gas_cost::SAR > gas_limit {
                        break; // should revert the tx
                    }
                    let shift = current_call_frame.stack.pop().unwrap();
                    let value = current_call_frame.stack.pop().unwrap();
                    let res = if shift < U256::from(256) {
                        arithmetic_shift_right(value, shift)
                    } else if value.bit(255) {
                        U256::MAX
                    } else {
                        U256::zero()
                    };
                    current_call_frame.stack.push(res);
                    consumed_gas += gas_cost::SAR
                }
                // DUPn
                op if (Opcode::DUP1..=Opcode::DUP16).contains(&op) => {
                    if consumed_gas + gas_cost::DUPN > gas_limit {
                        break; // should revert the tx
                    }
                    let depth = (op as u8) - (Opcode::DUP1 as u8) + 1;
                    assert!(
                        current_call_frame.stack.len().ge(&(depth as usize)),
                        "stack underflow: not enough values on the stack"
                    );
                    let value_at_depth = current_call_frame
                        .stack
                        .get(current_call_frame.stack.len() - depth as usize)
                        .unwrap();
                    current_call_frame.stack.push(*value_at_depth);
                    consumed_gas += gas_cost::DUPN
                }
                // SWAPn
                op if (Opcode::SWAP1..=Opcode::SWAP16).contains(&op) => {
                    if consumed_gas + gas_cost::SWAPN > gas_limit {
                        break; // should revert the tx
                    }
                    let depth = (op as u8) - (Opcode::SWAP1 as u8) + 1;
                    assert!(
                        current_call_frame.stack.len().ge(&(depth as usize)),
                        "stack underflow: not enough values on the stack"
                    );
                    let stack_top_index = current_call_frame.stack.len();
                    let to_swap_index = stack_top_index.checked_sub(depth as usize).unwrap();
                    current_call_frame
                        .stack
                        .swap(stack_top_index - 1, to_swap_index - 1);
                    consumed_gas += gas_cost::SWAPN
                }
                Opcode::POP => {
                    if consumed_gas + gas_cost::POP > gas_limit {
                        break; // should revert the tx
                    }
                    current_call_frame.stack.pop().unwrap();
                    consumed_gas += gas_cost::POP
                }
                Opcode::MLOAD => {
                    let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let memory_expansion_cost =
                        current_call_frame.memory.expansion_cost(offset + WORD_SIZE);
                    let gas_cost = gas_cost::MLOAD_STATIC + memory_expansion_cost;
                    if consumed_gas + gas_cost > gas_limit {
                        break; // should revert the tx
                    }

                    let value = current_call_frame.memory.load(offset);
                    current_call_frame.stack.push(value);
                    consumed_gas += gas_cost
                }
                Opcode::MSTORE => {
                    let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let memory_expansion_cost =
                        current_call_frame.memory.expansion_cost(offset + WORD_SIZE);
                    let gas_cost = gas_cost::MSTORE_STATIC + memory_expansion_cost;
                    if consumed_gas + gas_cost > gas_limit {
                        break; // should revert the tx
                    }

                    let value = current_call_frame.stack.pop().unwrap();
                    let mut value_bytes = [0u8; WORD_SIZE];
                    value.to_big_endian(&mut value_bytes);

                    current_call_frame.memory.store_bytes(offset, &value_bytes);
                    consumed_gas += gas_cost
                }
                Opcode::MSTORE8 => {
                    let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let memory_expansion_cost =
                        current_call_frame.memory.expansion_cost(offset + 1);
                    let gas_cost = gas_cost::MSTORE8_STATIC + memory_expansion_cost;
                    if consumed_gas + gas_cost > gas_limit {
                        break; // should revert the tx
                    }

                    let value = current_call_frame.stack.pop().unwrap();
                    let mut value_bytes = [0u8; WORD_SIZE];
                    value.to_big_endian(&mut value_bytes);

                    current_call_frame
                        .memory
                        .store_bytes(offset, value_bytes[WORD_SIZE - 1..WORD_SIZE].as_ref());
                    consumed_gas += gas_cost
                }
                Opcode::MSIZE => {
                    if consumed_gas + gas_cost::MSIZE > gas_limit {
                        break; // should revert the tx
                    }
                    current_call_frame
                        .stack
                        .push(current_call_frame.memory.size());
                    consumed_gas += gas_cost::MSIZE
                }
                Opcode::GAS => {
                    if consumed_gas + gas_cost::GAS > gas_limit {
                        break; // should revert the tx
                    }
                    let remaining_gas = gas_limit - consumed_gas - gas_cost::GAS;
                    current_call_frame.stack.push(remaining_gas.into());
                    consumed_gas += gas_cost::GAS
                }
                Opcode::MCOPY => {
                    let dest_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let src_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let size = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    if size == 0 {
                        continue;
                    }
                    let words_copied = (size + WORD_SIZE - 1) / WORD_SIZE;
                    let memory_byte_size = ((src_offset + size) as usize).max(dest_offset + size);
                    let memory_expansion_cost =
                        current_call_frame.memory.expansion_cost(memory_byte_size);
                    let gas_cost = gas_cost::MCOPY_STATIC
                        + gas_cost::MCOPY_DYNAMIC_BASE * words_copied as u64
                        + memory_expansion_cost;

                    current_call_frame
                        .memory
                        .copy(src_offset, dest_offset, size);
                    consumed_gas += gas_cost
                }
                _ => unimplemented!(),
            }
        }
    }

    pub fn current_call_frame(&mut self) -> &mut CallFrame {
        self.call_frames.last_mut().unwrap()
    }
}

pub fn arithmetic_shift_right(value: U256, shift: U256) -> U256 {
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
