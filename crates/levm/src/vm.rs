use std::{collections::HashMap, str::FromStr};

use crate::{
    block::{BlockEnv, LAST_AVAILABLE_BLOCK_LIMIT}, call_frame::{CallFrame, Log}, constants::{HALT_FOR_CALL, REVERT_FOR_CALL, SUCCESS_FOR_CALL, SUCCESS_FOR_RETURN}, opcodes::Opcode, primitives::{Address, Bytes, H256, H32, U256, U512}, transaction::{TransactTo, TxEnv}, vm_result::{ExecutionResult, ResultAndState, ResultReason, VMError}
};
use sha3::{Digest, Keccak256};

#[derive(Clone, Default, Debug, PartialEq, Eq)]
// TODO: complete account abstraction
pub struct Account {
    pub address: Address,
    pub balance: U256,
    pub bytecode: Bytes,
    pub storage: HashMap<U256, StorageSlot>,
    pub nonce: U256,
}

impl Account {
    pub fn new(balance: U256, bytecode: Bytes) -> Self {
        Self { balance, bytecode, ..Default::default() }
    }

    pub fn new_from(address: Address, balance: U256, bytecode: Bytes, storage: HashMap<U256, StorageSlot>, nonce: U256) -> Self {
        Self { address, balance, bytecode, storage, nonce }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StorageSlot {
    pub original_value: U256,
    pub current_value: U256,
}

#[derive(Debug, Clone, Default)]
pub struct Db {
    pub accounts: HashMap<Address, Account>,
    pub block_hashes: HashMap<U256, H256>,
}

impl Db {
    pub fn read_account_storage(&self, address: &Address, key: &U256) -> Option<StorageSlot> {
        self.accounts
            .get(address)
            .and_then(|account| account.storage.get(key))
            .cloned()
    }
    pub fn write_account_storage(&mut self, address: &Address, key: U256, slot: StorageSlot) {
        self.accounts
            .entry(*address)
            .or_default()
            .storage
            .insert(key, slot);
    }

    fn get_account_bytecode(&self, address: &Address) -> Bytes {
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

#[derive(Debug, Clone, Default)]
struct Substate; // TODO

#[derive(Debug, Default, Clone)]
pub struct Environment {
    /// The sender address of the transaction that originated
    /// this execution.
    // origin: Address,
    /// The price of gas paid by the signer of the transaction
    /// that originated this execution.
    // gas_price: u64,
    gas_limit: u64,
    pub consumed_gas: u64,
    /// The block header of the present block.
    pub block: BlockEnv,
}

#[derive(Debug, Default)]
/// Message, stuff needed for a call frame
pub struct Message {
    /// The address of the account which caused the
    /// code to be executing; if the execution agent is a
    /// transaction, this would be the transaction sender.
    pub msg_sender: Address,
    pub to: Address,
    /// The address of the account which owns the code that
    /// is executing.
    pub code_address: Address,
    pub delegate: Option<Address>,
    /// The byte array that is the input data to this execution;
    /// if the execution agent is a transaction, this would be
    /// the transaction data.
    pub data: Bytes,
    /// The value, in Wei, passed to this account as part
    /// of the same procedure as execution; if the execution
    /// agent is a transaction, this would be the transaction
    /// value.
    pub value: U256,
    /// The byte array that is the machine code to be executed.
    pub code: Bytes,
    /// The depth of the present message-call or
    /// contract-creation.
    pub depth: u16,
    pub gas: U256,
    pub is_static: bool,
}

#[derive(Debug, Clone, Default)]
pub struct VM {
    call_frames: Vec<CallFrame>,
    pub env: Environment,
    /// Information that is acted upon immediately following the
    /// transaction.
    pub accrued_substate: Substate,
    /// Mapping between addresses (160-bit identifiers) and account
    /// states.
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
    // TODO: block and transaction, not this
    pub fn new(tx_env: TxEnv, block_env: BlockEnv, db: Db) -> Self {
        // TxEnv {
        //     msg_sender,
        //     gas_limit: transaction.gas_limit[0].as_u64(),
        //     gas_price: transaction.gas_price,
        //     transact_to,
        //     value: transaction.value[0],
        //     chain_id: 0,
        //     data: decode_hex(transaction.data[0].clone()).unwrap(),
        //     nonce: Some(transaction.nonce.as_u64()),
        //     chain_id: 0,
        //     access_list: transaction.access_lists.get(0).cloned().flatten(),
        //     max_priority_fee_per_gas: transaction.max_priority_fee_per_gas,
        //     blob_hashes: transaction.blob_versioned_hashes.clone(),
        //     max_fee_per_blob_gas: transaction.max_fee_per_gas,
        // }

        let bytecode = match tx_env.transact_to {
            TransactTo::Call(addr) => db.get_account_bytecode(&addr),
            TransactTo::Create => {
                todo!()
            }
        };

        let to = match tx_env.transact_to {
            TransactTo::Call(addr) => addr,
            TransactTo::Create => Address::zero(),
        };

        let initial_call_frame = CallFrame::new(
            tx_env.msg_sender,
            to,
            tx_env.msg_sender,
            None,
            bytecode,
            tx_env.value,
            tx_env.data,
            U256::zero(),
            false,
            0,
        );

        let env = Environment {
            block: block_env,
            consumed_gas: TX_BASE_COST,
            gas_limit: u64::MAX,
        };

        Self {
            call_frames: vec![initial_call_frame],
            db,
            env,
            accrued_substate: Substate::default(),
        }
    }

    pub fn write_success_result(call_frame: CallFrame, reason: ResultReason) -> ExecutionResult {
        ExecutionResult::Success {
            reason,
            logs: call_frame.logs.clone(),
            return_data: call_frame.returndata.clone(),
        }
    }

    pub fn get_result(&self) -> Result<ResultAndState, VMError> {
        let gas_remaining = self.inner_context.gas_remaining.unwrap_or(0);
        let gas_initial = self.initial_gas;
        // TODO: Probably here we need to add the access_list_cost to gas_used, but we need a refactor of most tests
        let gas_used = gas_initial.saturating_sub(gas_remaining);
        let gas_refunded = self
            .inner_context
            .gas_refund
            .min(gas_used / GAS_REFUND_DENOMINATOR);
        let exit_status = self
            .inner_context
            .exit_status
            .clone()
            .unwrap_or(ExitStatusCode::Default);
        let return_values = self.return_values().to_vec();
        let halt_reason = self.halt_reason.unwrap_or(HaltReason::OpcodeNotFound);
        let result = match exit_status {
            ExitStatusCode::Return => ExecutionResult::Success {
                reason: SuccessReason::Return,
                gas_used,
                gas_refunded,
                output: Output::Call(return_values.into()), // TODO: add case Output::Create
                logs: self.logs(),
            },
            ExitStatusCode::Stop => ExecutionResult::Success {
                reason: SuccessReason::Stop,
                gas_used,
                gas_refunded,
                output: Output::Call(return_values.into()), // TODO: add case Output::Create
                logs: self.logs(),
            },
            ExitStatusCode::Revert => ExecutionResult::Revert {
                output: return_values.into(),
                gas_used,
            },
            ExitStatusCode::Error | ExitStatusCode::Default => ExecutionResult::Halt {
                reason: halt_reason,
                gas_used,
            },
        };

        // TODO: Check if this is ok
        let state = self.journal.into_state();

        Ok(ResultAndState { result, state })
    }

    pub fn execute(&mut self) -> Result<ExecutionResult, VMError> {  
        // let initial_gas_consumed = self.validate_transaction()?;
        let block_env = self.env.block.clone();
        let mut current_call_frame = self.call_frames.pop().ok_or(VMError::FatalError)?; // if this happens during execution, we are cooked 💀
        loop {
            let opcode = current_call_frame.next_opcode().unwrap_or(Opcode::STOP);
            match opcode {
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
                Opcode::CALLDATALOAD => {
                    let offset: usize = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let value = U256::from_big_endian(
                        &current_call_frame.calldata.slice(offset..offset + 32),
                    );
                    current_call_frame.stack.push(value);
                }
                Opcode::CALLDATASIZE => {
                    current_call_frame
                        .stack
                        .push(U256::from(current_call_frame.calldata.len()));
                }
                Opcode::CALLDATACOPY => {
                    let dest_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let calldata_offset: usize =
                        current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let size: usize = current_call_frame.stack.pop().unwrap().try_into().unwrap();
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
                        .push(U256::from(current_call_frame.returndata.len()));
                }
                Opcode::RETURNDATACOPY => {
                    let dest_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let returndata_offset: usize =
                        current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let size: usize = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    if size == 0 {
                        continue;
                    }
                    let data = current_call_frame
                        .returndata
                        .slice(returndata_offset..returndata_offset + size);
                    current_call_frame.memory.store_bytes(dest_offset, &data);
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
                        < self.env.block
                            .number
                            .saturating_sub(U256::from(LAST_AVAILABLE_BLOCK_LIMIT))
                        || block_number >= self.env.block.number
                    {
                        current_call_frame.stack.push(U256::zero());
                        continue;
                    }

                    if let Some(block_hash) = self.db.block_hashes.get(&block_number) {
                        current_call_frame
                            .stack
                            .push(U256::from_big_endian(&block_hash.0));
                    } else {
                        current_call_frame.stack.push(U256::zero());
                    };
                }
                Opcode::COINBASE => {
                    let coinbase = self.env.block.coinbase;
                    current_call_frame.stack.push(address_to_word(coinbase));
                }
                Opcode::TIMESTAMP => {
                    let timestamp = self.env.block.timestamp;
                    current_call_frame.stack.push(timestamp);
                }
                Opcode::NUMBER => {
                    let block_number = self.env.block.number;
                    current_call_frame.stack.push(block_number);
                }
                Opcode::PREVRANDAO => {
                    let randao = self.env.block.prev_randao.unwrap_or_default();
                    current_call_frame
                        .stack
                        .push(U256::from_big_endian(randao.0.as_slice()));
                }
                Opcode::GASLIMIT => {
                    let gas_limit = self.env.block.gas_limit;
                    current_call_frame.stack.push(U256::from(gas_limit));
                }
                Opcode::CHAINID => {
                    let chain_id = self.env.block.chain_id;
                    current_call_frame.stack.push(U256::from(chain_id));
                }
                Opcode::SELFBALANCE => {
                    todo!("when we have accounts implemented")
                }
                Opcode::BASEFEE => {
                    let base_fee = self.env.block.base_fee_per_gas;
                    current_call_frame.stack.push(base_fee);
                }
                Opcode::BLOBHASH => {
                    todo!("when we have tx implemented");
                }
                Opcode::BLOBBASEFEE => {
                    let blob_base_fee = self.env.block.calculate_blob_gas_price();
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
                op if (Opcode::LOG0..=Opcode::LOG4).contains(&op) => {
                    if current_call_frame.is_static {
                        panic!("Cannot create log in static context"); // should return an error and halt
                    }

                    let number_of_topics = (op as u8) - (Opcode::LOG0 as u8);
                    let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let size = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let topics = (0..number_of_topics)
                        .map(|_| {
                            let topic = current_call_frame.stack.pop().unwrap().as_u32();
                            H32::from_slice(topic.to_be_bytes().as_ref())
                        })
                        .collect();

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
                    let code_address =
                        Address::from_low_u64_be(current_call_frame.stack.pop().unwrap().low_u64());
                    let value = current_call_frame.stack.pop().unwrap();
                    let args_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let args_size = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let ret_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let ret_size = current_call_frame.stack.pop().unwrap().try_into().unwrap();

                    let msg_sender = current_call_frame.msg_sender; // caller remains the msg_sender
                    let to = current_call_frame.to; // to remains the same
                    let is_static = current_call_frame.is_static;

                    self.generic_call(
                        &mut current_call_frame,
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
                    );
                }
                // Opcode::RETURN => {
                //     let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                //     let size = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                //     let return_data = current_call_frame.memory.load_range(offset, size).into();
                //     if let Some(mut parent_call_frame) = self.call_frames.pop() {
                //         if let (Some(_ret_offset), Some(_ret_size)) = (
                //             parent_call_frame.return_data_offset,
                //             parent_call_frame.return_data_size,
                //         ) {
                //             parent_call_frame.returndata = return_data;
                //         }
                //         parent_call_frame.stack.push(U256::from(SUCCESS_FOR_RETURN));
                //         parent_call_frame.return_data_offset = None;
                //         parent_call_frame.return_data_size = None;
                //         current_call_frame = parent_call_frame.clone();
                //     } else {
                //         excecution completed (?)
                //         current_call_frame
                //             .stack
                //             .push(U256::from(SUCCESS_FOR_RETURN));
                //         break;
                //     }
                // }
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
                _ => return Err(VMError::OpcodeNotFound),
            }
        }
    }

    pub fn transact(&mut self) -> Result<ResultAndState, VMError> {
        // let initial_gas_consumed = self.validate_transaction()?;
        self.execute();
        self.get_result()
    }

    pub fn current_call_frame_mut(&mut self) -> &mut CallFrame {
        self.call_frames.last_mut().unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn generic_call(
        &mut self,
        current_call_frame: &mut CallFrame,
        gas: U256,
        value: U256,
        msg_sender: Address,
        to: Address,
        code_address: Address,
        delegate: Option<Address>,
        _should_transfer_value: bool,
        is_static: bool,
        args_offset: usize,
        args_size: usize,
        ret_offset: usize,
        ret_size: usize,
    ) -> Result<(), VMError> {
        // check balance
        if self.db.balance(&current_call_frame.msg_sender) < value {
            current_call_frame.stack.push(U256::from(REVERT_FOR_CALL))?;
            return Ok(());
        }

        // transfer value
        // transfer(&current_call_frame.msg_sender, &address, value);

        let code_address_bytecode = self.db.get_account_bytecode(&code_address);
        if code_address_bytecode.is_empty() {
            current_call_frame.stack.push(U256::from(SUCCESS_FOR_CALL))?;
            return Ok(());
        }

        let calldata = current_call_frame
            .memory
            .load_range(args_offset, args_size)
            .into();

        let new_call_frame = CallFrame::new(
            msg_sender,
            to,
            code_address,
            delegate,
            code_address_bytecode,
            value,
            calldata,
            gas,
            is_static,
            current_call_frame.depth + 1,
        );

        current_call_frame.return_data_offset = Some(ret_offset);
        current_call_frame.return_data_size = Some(ret_size);

        // self.call_frames.push(current_call_frame.clone());
        // *current_call_frame = new_call_frame;
        
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
            Ok(_) => {
                current_call_frame.stack.push(U256::from(HALT_FOR_CALL))?;
            }
            Err(_) => {
                current_call_frame.stack.push(U256::from(HALT_FOR_CALL))?;
            }
        };
        Ok(())
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
