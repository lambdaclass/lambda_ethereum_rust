use std::{collections::HashMap, str::FromStr};

use crate::{
    block::{BlockEnv, LAST_AVAILABLE_BLOCK_LIMIT},
    call_frame::CallFrame,
    constants::{
        MAX_CODE_SIZE, REVERT_FOR_CALL, REVERT_FOR_CREATE, SUCCESS_FOR_CALL, SUCCESS_FOR_RETURN,
    },
    opcodes::Opcode,
};
extern crate ethereum_rust_rlp;
use bytes::Bytes;
use ethereum_rust_rlp::encode::RLPEncode;
use ethereum_types::{Address, H160, H256, U256, U512};
use sha3::{Digest, Keccak256};

#[derive(Clone, Default, Debug)]
pub struct Account {
    pub balance: U256,
    pub bytecode: Bytes,
    pub nonce: u64,
}

impl Account {
    pub fn new(balance: U256, bytecode: Bytes, nonce: u64) -> Self {
        Self {
            balance,
            bytecode,
            nonce,
        }
    }
}

pub type Db = HashMap<U256, H256>;

#[derive(Debug, Clone, Default)]
pub struct VM {
    pub call_frames: Vec<CallFrame>,
    pub accounts: HashMap<Address, Account>, // change to Address
    pub block_env: BlockEnv,
    pub db: Db,
}

/// Shifts the value to the right by 255 bits and checks the most significant bit is a 1
fn is_negative(value: U256) -> bool {
    value.bit(255)
}
/// negates a number in two's complement
fn negate(value: U256) -> U256 {
    !value + U256::one()
}

fn address_to_word(address: Address) -> U256 {
    // This unwrap can't panic, as Address are 20 bytes long and U256 use 32 bytes
    U256::from_str(&format!("{address:?}")).unwrap()
}

impl VM {
    pub fn new(bytecode: Bytes, address: Address, balance: U256) -> Self {
        let initial_account = Account::new(balance, bytecode.clone(), 0);

        let initial_call_frame = CallFrame::new(bytecode);
        let mut accounts = HashMap::new();
        accounts.insert(address, initial_account);
        Self {
            call_frames: vec![initial_call_frame.clone()],
            accounts,
            block_env: Default::default(),
            db: Default::default(),
        }
    }

    pub fn execute(&mut self) {
        let block_env = self.block_env.clone();
        let mut current_call_frame = self.call_frames.pop().unwrap();
        loop {
            match current_call_frame.next_opcode().unwrap() {
                Opcode::STOP => break,
                Opcode::ADD => {
                    let augend = current_call_frame.stack.pop().unwrap();
                    let addend = current_call_frame.stack.pop().unwrap();
                    let sum = augend.overflowing_add(addend).0;
                    current_call_frame.stack.push(sum);
                }
                Opcode::MUL => {
                    let multiplicand = current_call_frame.stack.pop().unwrap();
                    let multiplier = current_call_frame.stack.pop().unwrap();
                    let product = multiplicand.overflowing_mul(multiplier).0;
                    current_call_frame.stack.push(product);
                }
                Opcode::SUB => {
                    let minuend = current_call_frame.stack.pop().unwrap();
                    let subtrahend = current_call_frame.stack.pop().unwrap();
                    let difference = minuend.overflowing_sub(subtrahend).0;
                    current_call_frame.stack.push(difference);
                }
                Opcode::DIV => {
                    let dividend = current_call_frame.stack.pop().unwrap();
                    let divisor = current_call_frame.stack.pop().unwrap();
                    if divisor.is_zero() {
                        current_call_frame.stack.push(U256::zero());
                        continue;
                    }
                    let quotient = dividend / divisor;
                    current_call_frame.stack.push(quotient);
                }
                Opcode::SDIV => {
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
                }
                Opcode::MOD => {
                    let dividend = current_call_frame.stack.pop().unwrap();
                    let divisor = current_call_frame.stack.pop().unwrap();
                    if divisor.is_zero() {
                        current_call_frame.stack.push(U256::zero());
                        continue;
                    }
                    let remainder = dividend % divisor;
                    current_call_frame.stack.push(remainder);
                }
                Opcode::SMOD => {
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
                }
                Opcode::ADDMOD => {
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
                }
                Opcode::MULMOD => {
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
                }
                Opcode::EXP => {
                    let base = current_call_frame.stack.pop().unwrap();
                    let exponent = current_call_frame.stack.pop().unwrap();
                    let power = base.overflowing_pow(exponent).0;
                    current_call_frame.stack.push(power);
                }
                Opcode::SIGNEXTEND => {
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
                }
                Opcode::LT => {
                    let lho = current_call_frame.stack.pop().unwrap();
                    let rho = current_call_frame.stack.pop().unwrap();
                    let result = if lho < rho { U256::one() } else { U256::zero() };
                    current_call_frame.stack.push(result);
                }
                Opcode::GT => {
                    let lho = current_call_frame.stack.pop().unwrap();
                    let rho = current_call_frame.stack.pop().unwrap();
                    let result = if lho > rho { U256::one() } else { U256::zero() };
                    current_call_frame.stack.push(result);
                }
                Opcode::SLT => {
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
                }
                Opcode::SGT => {
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
                }
                Opcode::EQ => {
                    let lho = current_call_frame.stack.pop().unwrap();
                    let rho = current_call_frame.stack.pop().unwrap();
                    let result = if lho == rho {
                        U256::one()
                    } else {
                        U256::zero()
                    };
                    current_call_frame.stack.push(result);
                }
                Opcode::ISZERO => {
                    let operand = current_call_frame.stack.pop().unwrap();
                    let result = if operand == U256::zero() {
                        U256::one()
                    } else {
                        U256::zero()
                    };
                    current_call_frame.stack.push(result);
                }
                Opcode::KECCAK256 => {
                    let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let size = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let value_bytes = current_call_frame.memory.load_range(offset, size);

                    let mut hasher = Keccak256::new();
                    hasher.update(value_bytes);
                    let result = hasher.finalize();
                    current_call_frame
                        .stack
                        .push(U256::from_big_endian(&result));
                }
                Opcode::JUMP => {
                    let jump_address = current_call_frame.stack.pop().unwrap();
                    current_call_frame.jump(jump_address);
                }
                Opcode::JUMPI => {
                    let jump_address = current_call_frame.stack.pop().unwrap();
                    let condition = current_call_frame.stack.pop().unwrap();
                    if condition != U256::zero() {
                        current_call_frame.jump(jump_address);
                    }
                }
                Opcode::JUMPDEST => {
                    // just consume some gas, jumptable written at the start
                }
                Opcode::PC => {
                    current_call_frame
                        .stack
                        .push(U256::from(current_call_frame.pc - 1));
                }
                Opcode::BLOCKHASH => {
                    let block_number = current_call_frame.stack.pop().unwrap();

                    // If number is not in the valid range (last 256 blocks), return zero.
                    if block_number
                        < block_env
                            .number
                            .saturating_sub(U256::from(LAST_AVAILABLE_BLOCK_LIMIT))
                        || block_number >= block_env.number
                    {
                        current_call_frame.stack.push(U256::zero());
                        continue;
                    }

                    if let Some(block_hash) = self.db.get(&block_number) {
                        current_call_frame
                            .stack
                            .push(U256::from_big_endian(&block_hash.0));
                    } else {
                        current_call_frame.stack.push(U256::zero());
                    };
                }
                Opcode::COINBASE => {
                    let coinbase = block_env.coinbase;
                    current_call_frame.stack.push(address_to_word(coinbase));
                }
                Opcode::TIMESTAMP => {
                    let timestamp = block_env.timestamp;
                    current_call_frame.stack.push(timestamp);
                }
                Opcode::NUMBER => {
                    let block_number = block_env.number;
                    current_call_frame.stack.push(block_number);
                }
                Opcode::PREVRANDAO => {
                    let randao = block_env.prev_randao.unwrap_or_default();
                    current_call_frame
                        .stack
                        .push(U256::from_big_endian(randao.0.as_slice()));
                }
                Opcode::GASLIMIT => {
                    let gas_limit = block_env.gas_limit;
                    current_call_frame.stack.push(U256::from(gas_limit));
                }
                Opcode::CHAINID => {
                    let chain_id = block_env.chain_id;
                    current_call_frame.stack.push(U256::from(chain_id));
                }
                Opcode::SELFBALANCE => {
                    todo!("when we have accounts implemented")
                }
                Opcode::BASEFEE => {
                    let base_fee = block_env.base_fee_per_gas;
                    current_call_frame.stack.push(base_fee);
                }
                Opcode::BLOBHASH => {
                    todo!("when we have tx implemented");
                }
                Opcode::BLOBBASEFEE => {
                    let blob_base_fee = block_env.calculate_blob_gas_price();
                    current_call_frame.stack.push(blob_base_fee);
                }
                Opcode::PUSH0 => {
                    current_call_frame.stack.push(U256::zero());
                }
                // PUSHn
                op if (Opcode::PUSH1..Opcode::PUSH32).contains(&op) => {
                    let n_bytes = (op as u8) - (Opcode::PUSH1 as u8) + 1;
                    let next_n_bytes = current_call_frame
                        .bytecode
                        .get(current_call_frame.pc()..current_call_frame.pc() + n_bytes as usize)
                        .expect("invalid bytecode");
                    let value_to_push = U256::from(next_n_bytes);
                    current_call_frame.stack.push(value_to_push);
                    current_call_frame.increment_pc_by(n_bytes as usize);
                }
                Opcode::PUSH32 => {
                    let next_32_bytes = current_call_frame
                        .bytecode
                        .get(current_call_frame.pc()..current_call_frame.pc() + 32)
                        .unwrap();
                    let value_to_push = U256::from(next_32_bytes);
                    current_call_frame.stack.push(value_to_push);
                    current_call_frame.increment_pc_by(32);
                }
                Opcode::AND => {
                    // spend_gas(3);
                    let a = current_call_frame.stack.pop().unwrap();
                    let b = current_call_frame.stack.pop().unwrap();
                    current_call_frame.stack.push(a & b);
                }
                Opcode::OR => {
                    // spend_gas(3);
                    let a = current_call_frame.stack.pop().unwrap();
                    let b = current_call_frame.stack.pop().unwrap();
                    current_call_frame.stack.push(a | b);
                }
                Opcode::XOR => {
                    // spend_gas(3);
                    let a = current_call_frame.stack.pop().unwrap();
                    let b = current_call_frame.stack.pop().unwrap();
                    current_call_frame.stack.push(a ^ b);
                }
                Opcode::NOT => {
                    // spend_gas(3);
                    let a = current_call_frame.stack.pop().unwrap();
                    current_call_frame.stack.push(!a);
                }
                Opcode::BYTE => {
                    // spend_gas(3);
                    let op1 = current_call_frame.stack.pop().unwrap();
                    let op2 = current_call_frame.stack.pop().unwrap();

                    let byte_index = op1.try_into().unwrap_or(usize::MAX);

                    if byte_index < 32 {
                        current_call_frame
                            .stack
                            .push(U256::from(op2.byte(31 - byte_index)));
                    } else {
                        current_call_frame.stack.push(U256::zero());
                    }
                }
                Opcode::SHL => {
                    // spend_gas(3);
                    let shift = current_call_frame.stack.pop().unwrap();
                    let value = current_call_frame.stack.pop().unwrap();
                    if shift < U256::from(256) {
                        current_call_frame.stack.push(value << shift);
                    } else {
                        current_call_frame.stack.push(U256::zero());
                    }
                }
                Opcode::SHR => {
                    // spend_gas(3);
                    let shift = current_call_frame.stack.pop().unwrap();
                    let value = current_call_frame.stack.pop().unwrap();
                    if shift < U256::from(256) {
                        current_call_frame.stack.push(value >> shift);
                    } else {
                        current_call_frame.stack.push(U256::zero());
                    }
                }
                Opcode::SAR => {
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
                }
                // DUPn
                op if (Opcode::DUP1..=Opcode::DUP16).contains(&op) => {
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
                }
                // SWAPn
                op if (Opcode::SWAP1..=Opcode::SWAP16).contains(&op) => {
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
                }
                Opcode::POP => {
                    current_call_frame.stack.pop().unwrap();
                }
                Opcode::MLOAD => {
                    // spend_gas(3);
                    let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let value = current_call_frame.memory.load(offset);
                    current_call_frame.stack.push(value);
                }
                Opcode::MSTORE => {
                    // spend_gas(3);
                    let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let value = current_call_frame.stack.pop().unwrap();
                    let mut value_bytes = [0u8; 32];
                    value.to_big_endian(&mut value_bytes);

                    current_call_frame.memory.store_bytes(offset, &value_bytes);
                }
                Opcode::MSTORE8 => {
                    // spend_gas(3);
                    let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let value = current_call_frame.stack.pop().unwrap();
                    let mut value_bytes = [0u8; 32];
                    value.to_big_endian(&mut value_bytes);

                    current_call_frame
                        .memory
                        .store_bytes(offset, value_bytes[31..32].as_ref());
                }
                Opcode::MSIZE => {
                    // spend_gas(2);
                    current_call_frame
                        .stack
                        .push(current_call_frame.memory.size());
                }
                Opcode::MCOPY => {
                    // spend_gas(3) + dynamic gas
                    let dest_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let src_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let size = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    if size == 0 {
                        continue;
                    }

                    current_call_frame
                        .memory
                        .copy(src_offset, dest_offset, size);
                }
                Opcode::CALL => {
                    let gas = current_call_frame.stack.pop().unwrap();
                    let address =
                        Address::from_low_u64_be(current_call_frame.stack.pop().unwrap().low_u64());
                    let value = current_call_frame.stack.pop().unwrap();
                    let args_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let args_size = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let ret_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let ret_size = current_call_frame.stack.pop().unwrap().try_into().unwrap();

                    // check balance
                    if self.balance(&current_call_frame.msg_sender) < value {
                        current_call_frame.stack.push(U256::from(REVERT_FOR_CALL));
                        continue;
                    }

                    // transfer value
                    // transfer(&current_call_frame.msg_sender, &address, value);

                    let callee_bytecode = self.get_account_bytecode(&address);

                    if callee_bytecode.is_empty() {
                        current_call_frame.stack.push(U256::from(SUCCESS_FOR_CALL));
                        continue;
                    }

                    let calldata = current_call_frame
                        .memory
                        .load_range(args_offset, args_size)
                        .into();

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

                    self.call_frames.push(current_call_frame.clone());
                    current_call_frame = new_call_frame;
                }
                Opcode::RETURN => {
                    let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let size = current_call_frame.stack.pop().unwrap().try_into().unwrap();

                    let return_data = current_call_frame.memory.load_range(offset, size);

                    if let Some(mut parent_call_frame) = self.call_frames.pop() {
                        if let (Some(ret_offset), Some(_ret_size)) = (
                            parent_call_frame.return_data_offset,
                            parent_call_frame.return_data_size,
                        ) {
                            parent_call_frame
                                .memory
                                .store_bytes(ret_offset, &return_data);
                        }

                        parent_call_frame.stack.push(U256::from(SUCCESS_FOR_RETURN));
                        parent_call_frame.return_data_offset = None;
                        parent_call_frame.return_data_size = None;

                        current_call_frame = parent_call_frame.clone();
                    } else {
                        // excecution completed (?)
                        current_call_frame
                            .stack
                            .push(U256::from(SUCCESS_FOR_RETURN));
                        break;
                    }
                }
                Opcode::CREATE => {
                    let value_in_wei_to_send = current_call_frame.stack.pop().unwrap();
                    let code_offset_in_memory =
                        current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let code_size_in_memory =
                        current_call_frame.stack.pop().unwrap().try_into().unwrap();

                    self.create_aux(
                        value_in_wei_to_send,
                        code_offset_in_memory,
                        code_size_in_memory,
                        None,
                        &mut current_call_frame,
                    );
                }
                Opcode::CREATE2 => {
                    let value_in_wei_to_send = current_call_frame.stack.pop().unwrap();
                    let code_offset_in_memory =
                        current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let code_size_in_memory =
                        current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let salt = current_call_frame.stack.pop().unwrap();

                    self.create_aux(
                        value_in_wei_to_send,
                        code_offset_in_memory,
                        code_size_in_memory,
                        Some(salt),
                        &mut current_call_frame,
                    );
                }
                Opcode::TLOAD => {
                    let key = current_call_frame.stack.pop().unwrap();
                    let value = current_call_frame
                        .transient_storage
                        .get(&(current_call_frame.msg_sender, key))
                        .cloned()
                        .unwrap_or(U256::zero());

                    current_call_frame.stack.push(value);
                }
                Opcode::TSTORE => {
                    let key = current_call_frame.stack.pop().unwrap();
                    let value = current_call_frame.stack.pop().unwrap();

                    current_call_frame
                        .transient_storage
                        .insert((current_call_frame.msg_sender, key), value);
                }
                _ => unimplemented!(),
            }
        }
        self.call_frames.push(current_call_frame);
    }

    pub fn current_call_frame_mut(&mut self) -> &mut CallFrame {
        self.call_frames.last_mut().unwrap()
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

    pub fn add_account(&mut self, address: Address, account: Account) {
        self.accounts.insert(address, account);
    }

    pub fn current_call_frame(&self) -> &CallFrame {
        self.call_frames.last().unwrap()
    }

    pub fn create_aux(
        &mut self,
        value_in_wei_to_send: U256,
        code_offset_in_memory: usize,
        code_size_in_memory: usize,
        salt: Option<U256>,
        current_call_frame: &mut CallFrame,
    ) {
        if code_size_in_memory > MAX_CODE_SIZE * 2 {
            current_call_frame.stack.push(U256::from(REVERT_FOR_CREATE));
            return;
        }
        if current_call_frame.is_static {
            current_call_frame.stack.push(U256::from(REVERT_FOR_CREATE));
            return;
        }

        let sender_account = self
            .accounts
            .get_mut(&current_call_frame.msg_sender)
            .unwrap();

        if sender_account.balance < value_in_wei_to_send {
            current_call_frame.stack.push(U256::from(REVERT_FOR_CREATE));
            return;
        }

        let Some(new_nonce) = sender_account.nonce.checked_add(1) else {
            current_call_frame.stack.push(U256::from(REVERT_FOR_CREATE));
            return;
        };
        sender_account.nonce = new_nonce; // should we increase the nonce here or after the calculation of the address?
        sender_account.balance -= value_in_wei_to_send;
        let code = Bytes::from(
            current_call_frame
                .memory
                .load_range(code_offset_in_memory, code_size_in_memory),
        );

        let new_address = match salt {
            Some(salt) => {
                Self::calculate_create2_address(current_call_frame.msg_sender, code.clone(), salt)
            }
            None => {
                Self::calculate_create_address(current_call_frame.msg_sender, sender_account.nonce)
            }
        };
        if self.accounts.contains_key(&new_address) {
            current_call_frame.stack.push(U256::from(REVERT_FOR_CREATE));
            return;
        }
        // nonce == 1, as we will execute this contract right now
        let new_account = Account::new(value_in_wei_to_send, code.clone(), 1);
        self.add_account(new_address, new_account);

        let mut gas = current_call_frame.gas;
        gas -= gas / 64; // 63/64 of the gas to the call
        current_call_frame.gas -= gas; // leaves 1/64  of the gas to current call frame

        let new_call_frame = CallFrame {
            gas,
            msg_sender: current_call_frame.msg_sender,
            callee: new_address,
            bytecode: code,
            msg_value: value_in_wei_to_send,
            calldata: Bytes::new(),
            ..Default::default()
        };

        current_call_frame.return_data_offset = Some(code_offset_in_memory);
        current_call_frame.return_data_size = Some(code_size_in_memory);
        current_call_frame.stack.push(address_to_word(new_address));
        self.call_frames.push(current_call_frame.clone());
        *current_call_frame = new_call_frame;
    }

    /// Calculates the address of a new conctract using the CREATE opcode as follow
    /// address = keccak256(rlp([sender_address,sender_nonce]))[12:]
    pub fn calculate_create_address(sender_address: Address, sender_nonce: u64) -> H160 {
        let mut encoded = Vec::new();
        sender_address.encode(&mut encoded);
        sender_nonce.encode(&mut encoded);
        let mut hasher = Keccak256::new();
        hasher.update(encoded);
        Address::from_slice(&hasher.finalize()[12..])
    }
    /// Calculates the address of a new contract using the CREATE2 opcode as follow
    /// initialization_code = memory[offset:offset+size]
    /// address = keccak256(0xff + sender_address + salt + keccak256(initialization_code))[12:]
    pub fn calculate_create2_address(
        sender_address: Address,
        initialization_code: Bytes,
        salt: U256,
    ) -> H160 {
        let mut hasher = Keccak256::new();
        hasher.update(initialization_code.clone());
        let initialization_code_hash = hasher.finalize();
        let mut hasher = Keccak256::new();
        let mut salt_bytes = [0; 32];
        salt.to_big_endian(&mut salt_bytes);
        hasher.update([0xff]);
        hasher.update(sender_address.as_bytes());
        hasher.update(salt_bytes);
        hasher.update(initialization_code_hash);
        Address::from_slice(&hasher.finalize()[12..])
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
