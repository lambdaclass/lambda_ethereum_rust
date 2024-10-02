use std::{collections::HashMap, str::FromStr};

use crate::{
    block::{BlockEnv, LAST_AVAILABLE_BLOCK_LIMIT},
    call_frame::{CallFrame, Log},
    constants::{HALT_FOR_CALL, REVERT_FOR_CALL, SUCCESS_FOR_CALL, SUCCESS_FOR_RETURN},
    opcodes::Opcode,
    vm_result::{ExecutionResult, ResultReason, VMError},
};
use bytes::Bytes;
use ethereum_types::{Address, H256, H32, U256, U512};
use sha3::{Digest, Keccak256};

#[derive(Clone, Default, Debug)]
pub struct Account {
    balance: U256,
    bytecode: Bytes,
}

impl Account {
    pub fn new(balance: U256, bytecode: Bytes) -> Self {
        Self { balance, bytecode }
    }
}

pub type Db = HashMap<U256, H256>;

#[derive(Debug, Clone, Default)]
pub struct VM {
    pub call_frames: Vec<CallFrame>,
    pub accounts: HashMap<Address, Account>,
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
        let initial_account = Account::new(balance, bytecode.clone());

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

    pub fn write_success_result(call_frame: CallFrame, reason: ResultReason) -> ExecutionResult {
        ExecutionResult::Success {
            reason,
            logs: call_frame.logs.clone(),
            return_data: call_frame.returndata.clone(),
        }
    }

    pub fn execute(&mut self) -> Result<ExecutionResult, VMError> {
        let block_env = self.block_env.clone();
        let mut current_call_frame = self.call_frames.pop().ok_or(VMError::FatalError)?; // if this happens during execution, we are cooked 💀
        loop {
            let opcode = current_call_frame.next_opcode().unwrap_or(Opcode::STOP);
            match opcode {
                Opcode::STOP => {
                    self.call_frames.push(current_call_frame.clone());
                    return Ok(Self::write_success_result(
                        current_call_frame,
                        ResultReason::Stop,
                    ));
                }
                Opcode::ADD => {
                    let augend = current_call_frame.stack.pop()?;
                    let addend = current_call_frame.stack.pop()?;
                    let sum = augend.overflowing_add(addend).0;
                    current_call_frame.stack.push(sum)?;
                }
                Opcode::MUL => {
                    let multiplicand = current_call_frame.stack.pop()?;
                    let multiplier = current_call_frame.stack.pop()?;
                    let product = multiplicand.overflowing_mul(multiplier).0;
                    current_call_frame.stack.push(product)?;
                }
                Opcode::SUB => {
                    let minuend = current_call_frame.stack.pop()?;
                    let subtrahend = current_call_frame.stack.pop()?;
                    let difference = minuend.overflowing_sub(subtrahend).0;
                    current_call_frame.stack.push(difference)?;
                }
                Opcode::DIV => {
                    let dividend = current_call_frame.stack.pop()?;
                    let divisor = current_call_frame.stack.pop()?;
                    if divisor.is_zero() {
                        current_call_frame.stack.push(U256::zero())?;
                        continue;
                    }
                    let quotient = dividend / divisor;
                    current_call_frame.stack.push(quotient)?;
                }
                Opcode::SDIV => {
                    let dividend = current_call_frame.stack.pop()?;
                    let divisor = current_call_frame.stack.pop()?;
                    if divisor.is_zero() {
                        current_call_frame.stack.push(U256::zero())?;
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

                    current_call_frame.stack.push(quotient)?;
                }
                Opcode::MOD => {
                    let dividend = current_call_frame.stack.pop()?;
                    let divisor = current_call_frame.stack.pop()?;
                    if divisor.is_zero() {
                        current_call_frame.stack.push(U256::zero())?;
                        continue;
                    }
                    let remainder = dividend % divisor;
                    current_call_frame.stack.push(remainder)?;
                }
                Opcode::SMOD => {
                    let dividend = current_call_frame.stack.pop()?;
                    let divisor = current_call_frame.stack.pop()?;
                    if divisor.is_zero() {
                        current_call_frame.stack.push(U256::zero())?;
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

                    current_call_frame.stack.push(remainder)?;
                }
                Opcode::ADDMOD => {
                    let augend = current_call_frame.stack.pop()?;
                    let addend = current_call_frame.stack.pop()?;
                    let divisor = current_call_frame.stack.pop()?;
                    if divisor.is_zero() {
                        current_call_frame.stack.push(U256::zero())?;
                        continue;
                    }
                    let (sum, overflow) = augend.overflowing_add(addend);
                    let mut remainder = sum % divisor;
                    if overflow || remainder > divisor {
                        remainder = remainder.overflowing_sub(divisor).0;
                    }

                    current_call_frame.stack.push(remainder)?;
                }
                Opcode::MULMOD => {
                    let multiplicand = U512::from(current_call_frame.stack.pop()?);

                    let multiplier = U512::from(current_call_frame.stack.pop()?);
                    let divisor = U512::from(current_call_frame.stack.pop()?);
                    if divisor.is_zero() {
                        current_call_frame.stack.push(U256::zero())?;
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
                    current_call_frame.stack.push(remainder)?;
                }
                Opcode::EXP => {
                    let base = current_call_frame.stack.pop()?;
                    let exponent = current_call_frame.stack.pop()?;
                    let power = base.overflowing_pow(exponent).0;
                    current_call_frame.stack.push(power)?;
                }
                Opcode::SIGNEXTEND => {
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
                }
                Opcode::LT => {
                    let lho = current_call_frame.stack.pop()?;
                    let rho = current_call_frame.stack.pop()?;
                    let result = if lho < rho { U256::one() } else { U256::zero() };
                    current_call_frame.stack.push(result)?;
                }
                Opcode::GT => {
                    let lho = current_call_frame.stack.pop()?;
                    let rho = current_call_frame.stack.pop()?;
                    let result = if lho > rho { U256::one() } else { U256::zero() };
                    current_call_frame.stack.push(result)?;
                }
                Opcode::SLT => {
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
                }
                Opcode::SGT => {
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
                }
                Opcode::EQ => {
                    let lho = current_call_frame.stack.pop()?;
                    let rho = current_call_frame.stack.pop()?;
                    let result = if lho == rho {
                        U256::one()
                    } else {
                        U256::zero()
                    };
                    current_call_frame.stack.push(result)?;
                }
                Opcode::ISZERO => {
                    let operand = current_call_frame.stack.pop()?;
                    let result = if operand == U256::zero() {
                        U256::one()
                    } else {
                        U256::zero()
                    };
                    current_call_frame.stack.push(result)?;
                }
                Opcode::KECCAK256 => {
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
                    let value_bytes = current_call_frame.memory.load_range(offset, size);

                    let mut hasher = Keccak256::new();
                    hasher.update(value_bytes);
                    let result = hasher.finalize();
                    current_call_frame
                        .stack
                        .push(U256::from_big_endian(&result))?;
                }
                Opcode::CALLDATALOAD => {
                    let offset: usize = current_call_frame
                        .stack
                        .pop()?
                        .try_into()
                        .unwrap_or(usize::MAX);
                    let value = U256::from_big_endian(
                        &current_call_frame.calldata.slice(offset..offset + 32),
                    );
                    current_call_frame.stack.push(value)?;
                }
                Opcode::CALLDATASIZE => {
                    current_call_frame
                        .stack
                        .push(U256::from(current_call_frame.calldata.len()))?;
                }
                Opcode::CALLDATACOPY => {
                    let dest_offset = current_call_frame
                        .stack
                        .pop()?
                        .try_into()
                        .unwrap_or(usize::MAX);
                    let calldata_offset: usize = current_call_frame
                        .stack
                        .pop()?
                        .try_into()
                        .unwrap_or(usize::MAX);
                    let size: usize = current_call_frame
                        .stack
                        .pop()?
                        .try_into()
                        .unwrap_or(usize::MAX);
                    if size == 0 {
                        continue;
                    }
                    let data = current_call_frame
                        .calldata
                        .slice(calldata_offset..calldata_offset + size);

                    current_call_frame.memory.store_bytes(dest_offset, &data);
                }
                Opcode::RETURNDATASIZE => {
                    current_call_frame
                        .stack
                        .push(U256::from(current_call_frame.returndata.len()))?;
                }
                Opcode::RETURNDATACOPY => {
                    let dest_offset = current_call_frame
                        .stack
                        .pop()?
                        .try_into()
                        .unwrap_or(usize::MAX);
                    let returndata_offset: usize = current_call_frame
                        .stack
                        .pop()?
                        .try_into()
                        .unwrap_or(usize::MAX);
                    let size: usize = current_call_frame
                        .stack
                        .pop()?
                        .try_into()
                        .unwrap_or(usize::MAX);
                    if size == 0 {
                        continue;
                    }
                    let data = current_call_frame
                        .returndata
                        .slice(returndata_offset..returndata_offset + size);
                    current_call_frame.memory.store_bytes(dest_offset, &data);
                }
                Opcode::JUMP => {
                    let jump_address = current_call_frame.stack.pop()?;
                    if !current_call_frame.jump(jump_address) {
                        return Err(VMError::InvalidJump);
                    }
                }
                Opcode::JUMPI => {
                    let jump_address = current_call_frame.stack.pop()?;
                    let condition = current_call_frame.stack.pop()?;
                    if condition != U256::zero() && !current_call_frame.jump(jump_address) {
                        return Err(VMError::InvalidJump);
                    }
                }
                Opcode::JUMPDEST => {
                    // just consume some gas, jumptable written at the start
                }
                Opcode::PC => {
                    current_call_frame
                        .stack
                        .push(U256::from(current_call_frame.pc - 1))?;
                }
                Opcode::BLOCKHASH => {
                    let block_number = current_call_frame.stack.pop()?;

                    // If number is not in the valid range (last 256 blocks), return zero.
                    if block_number
                        < block_env
                            .number
                            .saturating_sub(U256::from(LAST_AVAILABLE_BLOCK_LIMIT))
                        || block_number >= block_env.number
                    {
                        current_call_frame.stack.push(U256::zero())?;
                        continue;
                    }

                    if let Some(block_hash) = self.db.get(&block_number) {
                        current_call_frame
                            .stack
                            .push(U256::from_big_endian(&block_hash.0))?;
                    } else {
                        current_call_frame.stack.push(U256::zero())?;
                    };
                }
                Opcode::COINBASE => {
                    let coinbase = block_env.coinbase;
                    current_call_frame.stack.push(address_to_word(coinbase))?;
                }
                Opcode::TIMESTAMP => {
                    let timestamp = block_env.timestamp;
                    current_call_frame.stack.push(timestamp)?;
                }
                Opcode::NUMBER => {
                    let block_number = block_env.number;
                    current_call_frame.stack.push(block_number)?;
                }
                Opcode::PREVRANDAO => {
                    let randao = block_env.prev_randao.unwrap_or_default();
                    current_call_frame
                        .stack
                        .push(U256::from_big_endian(randao.0.as_slice()))?;
                }
                Opcode::GASLIMIT => {
                    let gas_limit = block_env.gas_limit;
                    current_call_frame.stack.push(U256::from(gas_limit))?;
                }
                Opcode::CHAINID => {
                    let chain_id = block_env.chain_id;
                    current_call_frame.stack.push(U256::from(chain_id))?;
                }
                Opcode::SELFBALANCE => {
                    todo!("when we have accounts implemented")
                }
                Opcode::BASEFEE => {
                    let base_fee = block_env.base_fee_per_gas;
                    current_call_frame.stack.push(base_fee)?;
                }
                Opcode::BLOBHASH => {
                    todo!("when we have tx implemented");
                }
                Opcode::BLOBBASEFEE => {
                    let blob_base_fee = block_env.calculate_blob_gas_price();
                    current_call_frame.stack.push(blob_base_fee)?;
                }
                Opcode::PUSH0 => {
                    current_call_frame.stack.push(U256::zero())?;
                }
                // PUSHn
                op if (Opcode::PUSH1..Opcode::PUSH32).contains(&op) => {
                    let n_bytes = (op as u8) - (Opcode::PUSH1 as u8) + 1;
                    let next_n_bytes = current_call_frame
                        .bytecode
                        .get(current_call_frame.pc()..current_call_frame.pc() + n_bytes as usize)
                        .ok_or(VMError::InvalidBytecode)?; // this shouldn't really happen during execution
                    let value_to_push = U256::from(next_n_bytes);
                    current_call_frame.stack.push(value_to_push)?;
                    current_call_frame.increment_pc_by(n_bytes as usize);
                }
                Opcode::PUSH32 => {
                    let next_32_bytes = current_call_frame
                        .bytecode
                        .get(current_call_frame.pc()..current_call_frame.pc() + 32)
                        .ok_or(VMError::InvalidBytecode)?;
                    let value_to_push = U256::from(next_32_bytes);
                    current_call_frame.stack.push(value_to_push)?;
                    current_call_frame.increment_pc_by(32);
                }
                Opcode::AND => {
                    // spend_gas(3);
                    let a = current_call_frame.stack.pop()?;
                    let b = current_call_frame.stack.pop()?;
                    current_call_frame.stack.push(a & b)?;
                }
                Opcode::OR => {
                    // spend_gas(3);
                    let a = current_call_frame.stack.pop()?;
                    let b = current_call_frame.stack.pop()?;
                    current_call_frame.stack.push(a | b)?;
                }
                Opcode::XOR => {
                    // spend_gas(3);
                    let a = current_call_frame.stack.pop()?;
                    let b = current_call_frame.stack.pop()?;
                    current_call_frame.stack.push(a ^ b)?;
                }
                Opcode::NOT => {
                    // spend_gas(3);
                    let a = current_call_frame.stack.pop()?;
                    current_call_frame.stack.push(!a)?;
                }
                Opcode::BYTE => {
                    // spend_gas(3);
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
                }
                Opcode::SHL => {
                    // spend_gas(3);
                    let shift = current_call_frame.stack.pop()?;
                    let value = current_call_frame.stack.pop()?;
                    if shift < U256::from(256) {
                        current_call_frame.stack.push(value << shift)?;
                    } else {
                        current_call_frame.stack.push(U256::zero())?;
                    }
                }
                Opcode::SHR => {
                    // spend_gas(3);
                    let shift = current_call_frame.stack.pop()?;
                    let value = current_call_frame.stack.pop()?;
                    if shift < U256::from(256) {
                        current_call_frame.stack.push(value >> shift)?;
                    } else {
                        current_call_frame.stack.push(U256::zero())?;
                    }
                }
                Opcode::SAR => {
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
                }
                // DUPn
                op if (Opcode::DUP1..=Opcode::DUP16).contains(&op) => {
                    let depth = (op as u8) - (Opcode::DUP1 as u8) + 1;

                    if current_call_frame.stack.len() < depth as usize {
                        return Err(VMError::StackUnderflow);
                    }

                    let value_at_depth = current_call_frame
                        .stack
                        .get(current_call_frame.stack.len() - depth as usize)?;
                    current_call_frame.stack.push(*value_at_depth)?;
                }
                // SWAPn
                op if (Opcode::SWAP1..=Opcode::SWAP16).contains(&op) => {
                    let depth = (op as u8) - (Opcode::SWAP1 as u8) + 1;

                    if current_call_frame.stack.len() < depth as usize {
                        return Err(VMError::StackUnderflow);
                    }
                    let stack_top_index = current_call_frame.stack.len();
                    let to_swap_index = stack_top_index
                        .checked_sub(depth as usize)
                        .ok_or(VMError::StackUnderflow)?;
                    current_call_frame
                        .stack
                        .swap(stack_top_index - 1, to_swap_index - 1);
                }
                Opcode::POP => {
                    current_call_frame.stack.pop()?;
                }
                op if (Opcode::LOG0..=Opcode::LOG4).contains(&op) => {
                    if current_call_frame.is_static {
                        return Err(VMError::OpcodeNotAllowedInStaticContext);
                    }

                    let number_of_topics = (op as u8) - (Opcode::LOG0 as u8);
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
                    let mut topics = Vec::new();
                    for _ in 0..number_of_topics {
                        let topic = current_call_frame.stack.pop()?.as_u32();
                        topics.push(H32::from_slice(topic.to_be_bytes().as_ref()));
                    }

                    let data = current_call_frame.memory.load_range(offset, size);
                    let log = Log {
                        address: current_call_frame.msg_sender, // Should change the addr if we are on a Call/Create transaction (Call should be the contract we are calling, Create should be the original caller)
                        topics,
                        data: Bytes::from(data),
                    };
                    current_call_frame.logs.push(log);
                }
                Opcode::MLOAD => {
                    // spend_gas(3);
                    let offset = current_call_frame
                        .stack
                        .pop()?
                        .try_into()
                        .unwrap_or(usize::MAX);
                    let value = current_call_frame.memory.load(offset);
                    current_call_frame.stack.push(value)?;
                }
                Opcode::MSTORE => {
                    // spend_gas(3);
                    let offset = current_call_frame
                        .stack
                        .pop()?
                        .try_into()
                        .unwrap_or(usize::MAX);
                    let value = current_call_frame.stack.pop()?;
                    let mut value_bytes = [0u8; 32];
                    value.to_big_endian(&mut value_bytes);

                    current_call_frame.memory.store_bytes(offset, &value_bytes);
                }
                Opcode::MSTORE8 => {
                    // spend_gas(3);
                    let offset = current_call_frame
                        .stack
                        .pop()?
                        .try_into()
                        .unwrap_or(usize::MAX);
                    let value = current_call_frame.stack.pop()?;
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
                        .push(current_call_frame.memory.size())?;
                }
                Opcode::MCOPY => {
                    // spend_gas(3) + dynamic gas
                    let dest_offset = current_call_frame
                        .stack
                        .pop()?
                        .try_into()
                        .unwrap_or(usize::MAX);
                    let src_offset = current_call_frame
                        .stack
                        .pop()?
                        .try_into()
                        .unwrap_or(usize::MAX);
                    let size = current_call_frame
                        .stack
                        .pop()?
                        .try_into()
                        .unwrap_or(usize::MAX);
                    if size == 0 {
                        continue;
                    }
                    current_call_frame
                        .memory
                        .copy(src_offset, dest_offset, size);
                }
                Opcode::CALL => {
                    let gas = current_call_frame.stack.pop()?;
                    let address =
                        Address::from_low_u64_be(current_call_frame.stack.pop()?.low_u64());
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
                    // check balance
                    if self.balance(&current_call_frame.msg_sender) < value {
                        current_call_frame.stack.push(U256::from(REVERT_FOR_CALL))?;
                        continue;
                    }
                    // transfer value
                    // transfer(&current_call_frame.msg_sender, &address, value);
                    let callee_bytecode = self.get_account_bytecode(&address);
                    if callee_bytecode.is_empty() {
                        current_call_frame
                            .stack
                            .push(U256::from(SUCCESS_FOR_CALL))?;
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
                    self.call_frames.push(new_call_frame.clone());
                    let result = self.execute();

                    match result {
                        Ok(ExecutionResult::Success {
                            logs, return_data, ..
                        }) => {
                            current_call_frame.logs.extend(logs);
                            current_call_frame
                                .memory
                                .store_bytes(ret_offset, &return_data);
                            current_call_frame.returndata = return_data;
                            current_call_frame
                                .stack
                                .push(U256::from(SUCCESS_FOR_CALL))?;
                        }
                        Err(_) => {
                            current_call_frame.stack.push(U256::from(HALT_FOR_CALL))?;
                        }
                    };
                }
                Opcode::RETURN => {
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
                    let return_data = current_call_frame.memory.load_range(offset, size).into();

                    current_call_frame.returndata = return_data;
                    current_call_frame
                        .stack
                        .push(U256::from(SUCCESS_FOR_RETURN))?;
                    return Ok(Self::write_success_result(
                        current_call_frame,
                        ResultReason::Return,
                    ));
                }
                Opcode::TLOAD => {
                    let key = current_call_frame.stack.pop()?;
                    let value = current_call_frame
                        .transient_storage
                        .get(&(current_call_frame.msg_sender, key))
                        .cloned()
                        .unwrap_or(U256::zero());

                    current_call_frame.stack.push(value)?;
                }
                Opcode::TSTORE => {
                    let key = current_call_frame.stack.pop()?;
                    let value = current_call_frame.stack.pop()?;

                    current_call_frame
                        .transient_storage
                        .insert((current_call_frame.msg_sender, key), value);
                }
                _ => return Err(VMError::OpcodeNotFound),
            }
        }
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
