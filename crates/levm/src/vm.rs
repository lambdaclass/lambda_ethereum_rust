use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use crate::{
    block::{BlockEnv, LAST_AVAILABLE_BLOCK_LIMIT},
    call_frame::{CallFrame, Log},
    constants::*,
    opcodes::Opcode,
    primitives::{Address, Bytes, H256, H32, U256, U512},
    transaction::{TransactTo, TxEnv},
};
extern crate ethereum_rust_rlp;
use ethereum_rust_rlp::encode::RLPEncode;
use ethereum_types::H160;
use sha3::{Digest, Keccak256};

#[derive(Clone, Default, Debug)]
pub struct Account {
    pub balance: U256,
    pub bytecode: Bytes,
    pub nonce: u64,
    pub storage: HashMap<U256, StorageSlot>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StorageSlot {
    pub original_value: U256,
    pub current_value: U256,
}

impl Account {
    pub fn new(
        balance: U256,
        bytecode: Bytes,
        nonce: u64,
        storage: HashMap<U256, StorageSlot>,
    ) -> Self {
        Self {
            balance,
            bytecode,
            storage,
            nonce,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.balance.is_zero() && self.nonce == 0 && self.bytecode.is_empty()
    }

    pub fn with_balance(mut self, balance: U256) -> Self {
        self.balance = balance;
        self
    }

    pub fn with_bytecode(mut self, bytecode: Bytes) -> Self {
        self.bytecode = bytecode;
        self
    }

    pub fn with_storage(mut self, storage: HashMap<U256, StorageSlot>) -> Self {
        self.storage = storage;
        self
    }
}

pub type Storage = HashMap<U256, H256>;

#[derive(Clone, Debug, Default)]
pub struct Db {
    pub accounts: HashMap<Address, Account>,
    // contracts: HashMap<B256, Bytecode>,
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
// TODO: https://github.com/lambdaclass/ethereum_rust/issues/604
pub struct Substate {
    warm_addresses: HashSet<Address>,
}

/// Transaction environment shared by all the call frames
/// created by the current transaction.
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

// The execution model specifies how the system state is
// altered given a series of bytecode instructions and a small
// tuple of environmental data.

impl VM {
    pub fn new(tx_env: TxEnv, block_env: BlockEnv, db: Db) -> Self {
        let bytecode = match tx_env.transact_to {
            TransactTo::Call(addr) => db.get_account_bytecode(&addr),
            TransactTo::Create => {
                todo!()
            }
        };

        let caller = match tx_env.transact_to {
            TransactTo::Call(addr) => addr,
            TransactTo::Create => tx_env.msg_sender,
        };

        let code_addr = match tx_env.transact_to {
            TransactTo::Call(addr) => addr,
            TransactTo::Create => todo!(),
        };

        // TODO: this is mostly placeholder
        let initial_call_frame = CallFrame::new(
            U256::MAX,
            tx_env.msg_sender,
            caller,
            code_addr,
            Default::default(),
            bytecode,
            tx_env.value,
            tx_env.data,
            false,
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

    pub fn execute(&mut self) {
        let block_env = self.env.block.clone();
        let mut current_call_frame = self.call_frames.pop().unwrap();
        loop {
            let opcode = current_call_frame.next_opcode().unwrap_or(Opcode::STOP);
            match opcode {
                Opcode::STOP => break,
                Opcode::ADD => {
                    if self.env.consumed_gas + gas_cost::ADD > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let augend = current_call_frame.stack.pop().unwrap();
                    let addend = current_call_frame.stack.pop().unwrap();
                    let sum = augend.overflowing_add(addend).0;
                    current_call_frame.stack.push(sum);
                    self.env.consumed_gas += gas_cost::ADD
                }
                Opcode::MUL => {
                    if self.env.consumed_gas + gas_cost::MUL > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let multiplicand = current_call_frame.stack.pop().unwrap();
                    let multiplier = current_call_frame.stack.pop().unwrap();
                    let product = multiplicand.overflowing_mul(multiplier).0;
                    current_call_frame.stack.push(product);
                    self.env.consumed_gas += gas_cost::MUL
                }
                Opcode::SUB => {
                    if self.env.consumed_gas + gas_cost::SUB > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let minuend = current_call_frame.stack.pop().unwrap();
                    let subtrahend = current_call_frame.stack.pop().unwrap();
                    let difference = minuend.overflowing_sub(subtrahend).0;
                    current_call_frame.stack.push(difference);
                    self.env.consumed_gas += gas_cost::SUB
                }
                Opcode::DIV => {
                    if self.env.consumed_gas + gas_cost::DIV > self.env.gas_limit {
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
                    self.env.consumed_gas += gas_cost::DIV
                }
                Opcode::SDIV => {
                    if self.env.consumed_gas + gas_cost::SDIV > self.env.gas_limit {
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
                    self.env.consumed_gas += gas_cost::SDIV
                }
                Opcode::MOD => {
                    if self.env.consumed_gas + gas_cost::MOD > self.env.gas_limit {
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
                    self.env.consumed_gas += gas_cost::MOD
                }
                Opcode::SMOD => {
                    if self.env.consumed_gas + gas_cost::SMOD > self.env.gas_limit {
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
                    self.env.consumed_gas += gas_cost::SMOD
                }
                Opcode::ADDMOD => {
                    if self.env.consumed_gas + gas_cost::ADDMOD > self.env.gas_limit {
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
                    self.env.consumed_gas += gas_cost::ADDMOD
                }
                Opcode::MULMOD => {
                    if self.env.consumed_gas + gas_cost::MULMOD > self.env.gas_limit {
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
                    self.env.consumed_gas += gas_cost::MULMOD
                }
                Opcode::EXP => {
                    let base = current_call_frame.stack.pop().unwrap();
                    let exponent = current_call_frame.stack.pop().unwrap();

                    let exponent_byte_size = (exponent.bits() as u64 + 7) / 8;
                    let gas_cost =
                        gas_cost::EXP_STATIC + gas_cost::EXP_DYNAMIC_BASE * exponent_byte_size;
                    if self.env.consumed_gas + gas_cost > self.env.gas_limit {
                        break; // should revert the tx
                    }

                    let power = base.overflowing_pow(exponent).0;
                    current_call_frame.stack.push(power);
                    self.env.consumed_gas += gas_cost
                }
                Opcode::SIGNEXTEND => {
                    if self.env.consumed_gas + gas_cost::SIGNEXTEND > self.env.gas_limit {
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
                    self.env.consumed_gas += gas_cost::SIGNEXTEND
                }
                Opcode::LT => {
                    if self.env.consumed_gas + gas_cost::LT > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let lho = current_call_frame.stack.pop().unwrap();
                    let rho = current_call_frame.stack.pop().unwrap();
                    let result = if lho < rho { U256::one() } else { U256::zero() };
                    current_call_frame.stack.push(result);
                    self.env.consumed_gas += gas_cost::LT
                }
                Opcode::GT => {
                    if self.env.consumed_gas + gas_cost::GT > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let lho = current_call_frame.stack.pop().unwrap();
                    let rho = current_call_frame.stack.pop().unwrap();
                    let result = if lho > rho { U256::one() } else { U256::zero() };
                    current_call_frame.stack.push(result);
                    self.env.consumed_gas += gas_cost::GT
                }
                Opcode::SLT => {
                    if self.env.consumed_gas + gas_cost::SLT > self.env.gas_limit {
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
                    self.env.consumed_gas += gas_cost::SLT
                }
                Opcode::SGT => {
                    if self.env.consumed_gas + gas_cost::SGT > self.env.gas_limit {
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
                    self.env.consumed_gas += gas_cost::SGT
                }
                Opcode::EQ => {
                    if self.env.consumed_gas + gas_cost::EQ > self.env.gas_limit {
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
                    self.env.consumed_gas += gas_cost::EQ
                }
                Opcode::ISZERO => {
                    if self.env.consumed_gas + gas_cost::ISZERO > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let operand = current_call_frame.stack.pop().unwrap();
                    let result = if operand == U256::zero() {
                        U256::one()
                    } else {
                        U256::zero()
                    };
                    current_call_frame.stack.push(result);
                    self.env.consumed_gas += gas_cost::ISZERO
                }
                Opcode::KECCAK256 => {
                    let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let size = current_call_frame.stack.pop().unwrap().try_into().unwrap();

                    let minimum_word_size = (size + WORD_SIZE - 1) / WORD_SIZE;
                    let memory_expansion_cost =
                        current_call_frame.memory.expansion_cost(offset + size);
                    let gas_cost = gas_cost::KECCAK25_STATIC
                        + gas_cost::KECCAK25_DYNAMIC_BASE * minimum_word_size as u64
                        + memory_expansion_cost as u64;
                    if self.env.consumed_gas + gas_cost > self.env.gas_limit {
                        break; // should revert the tx
                    }

                    let value_bytes = current_call_frame.memory.load_range(offset, size);

                    let mut hasher = Keccak256::new();
                    hasher.update(value_bytes);
                    let result = hasher.finalize();
                    current_call_frame
                        .stack
                        .push(U256::from_big_endian(&result));
                    self.env.consumed_gas += gas_cost
                }
                Opcode::CALLDATALOAD => {
                    if self.env.consumed_gas + gas_cost::CALLDATALOAD > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let offset: usize = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let value = U256::from_big_endian(
                        &current_call_frame.calldata.slice(offset..offset + 32),
                    );
                    current_call_frame.stack.push(value);
                    self.env.consumed_gas += gas_cost::CALLDATALOAD
                }
                Opcode::CALLDATASIZE => {
                    if self.env.consumed_gas + gas_cost::CALLDATASIZE > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    current_call_frame
                        .stack
                        .push(U256::from(current_call_frame.calldata.len()));
                    self.env.consumed_gas += gas_cost::CALLDATASIZE
                }
                Opcode::CALLDATACOPY => {
                    let dest_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let calldata_offset: usize =
                        current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let size: usize = current_call_frame.stack.pop().unwrap().try_into().unwrap();

                    let minimum_word_size = (size + WORD_SIZE - 1) / WORD_SIZE;
                    let memory_expansion_cost =
                        current_call_frame.memory.expansion_cost(dest_offset + size) as u64;
                    let gas_cost = gas_cost::CALLDATACOPY_STATIC
                        + gas_cost::CALLDATACOPY_DYNAMIC_BASE * minimum_word_size as u64
                        + memory_expansion_cost;
                    if self.env.consumed_gas + gas_cost > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    self.env.consumed_gas += gas_cost;
                    if size == 0 {
                        continue;
                    }
                    let data = current_call_frame
                        .calldata
                        .slice(calldata_offset..calldata_offset + size);

                    current_call_frame.memory.store_bytes(dest_offset, &data);
                }
                Opcode::RETURNDATASIZE => {
                    if self.env.consumed_gas + gas_cost::RETURNDATASIZE > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    current_call_frame
                        .stack
                        .push(U256::from(current_call_frame.returndata.len()));
                    self.env.consumed_gas += gas_cost::RETURNDATASIZE
                }
                Opcode::RETURNDATACOPY => {
                    let dest_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let returndata_offset: usize =
                        current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let size: usize = current_call_frame.stack.pop().unwrap().try_into().unwrap();

                    let minimum_word_size = (size + WORD_SIZE - 1) / WORD_SIZE;
                    let memory_expansion_cost =
                        current_call_frame.memory.expansion_cost(dest_offset + size) as u64;
                    let gas_cost = gas_cost::RETURNDATACOPY_STATIC
                        + gas_cost::RETURNDATACOPY_DYNAMIC_BASE * minimum_word_size as u64
                        + memory_expansion_cost;
                    if self.env.consumed_gas + gas_cost > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    self.env.consumed_gas += gas_cost;
                    if size == 0 {
                        continue;
                    }
                    let data = current_call_frame
                        .returndata
                        .slice(returndata_offset..returndata_offset + size);
                    current_call_frame.memory.store_bytes(dest_offset, &data);
                }
                Opcode::JUMP => {
                    if self.env.consumed_gas + gas_cost::JUMP > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let jump_address = current_call_frame.stack.pop().unwrap();
                    current_call_frame.jump(jump_address);
                    self.env.consumed_gas += gas_cost::JUMP
                }
                Opcode::JUMPI => {
                    if self.env.consumed_gas + gas_cost::JUMPI > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let jump_address = current_call_frame.stack.pop().unwrap();
                    let condition = current_call_frame.stack.pop().unwrap();
                    if condition != U256::zero() {
                        current_call_frame.jump(jump_address);
                    }
                    self.env.consumed_gas += gas_cost::JUMPI
                }
                Opcode::JUMPDEST => {
                    // just consume some gas, jumptable written at the start
                    if self.env.consumed_gas + gas_cost::JUMPDEST > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    self.env.consumed_gas += gas_cost::JUMPDEST
                }
                Opcode::PC => {
                    if self.env.consumed_gas + gas_cost::PC > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    current_call_frame
                        .stack
                        .push(U256::from(current_call_frame.pc - 1));
                    self.env.consumed_gas += gas_cost::PC
                }
                Opcode::BLOCKHASH => {
                    if self.env.consumed_gas + gas_cost::BLOCKHASH > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let block_number = current_call_frame.stack.pop().unwrap();

                    self.env.consumed_gas += gas_cost::BLOCKHASH;
                    // If number is not in the valid range (last 256 blocks), return zero.
                    if block_number
                        < self
                            .env
                            .block
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
                    if self.env.consumed_gas + gas_cost::COINBASE > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let coinbase = block_env.coinbase;
                    current_call_frame.stack.push(address_to_word(coinbase));
                    self.env.consumed_gas += gas_cost::COINBASE
                }
                Opcode::TIMESTAMP => {
                    if self.env.consumed_gas + gas_cost::TIMESTAMP > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let timestamp = block_env.timestamp;
                    current_call_frame.stack.push(timestamp);
                    self.env.consumed_gas += gas_cost::TIMESTAMP
                }
                Opcode::NUMBER => {
                    if self.env.consumed_gas + gas_cost::NUMBER > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let block_number = block_env.number;
                    current_call_frame.stack.push(block_number);
                    self.env.consumed_gas += gas_cost::NUMBER
                }
                Opcode::PREVRANDAO => {
                    if self.env.consumed_gas + gas_cost::PREVRANDAO > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let randao = block_env.prev_randao.unwrap_or_default();
                    current_call_frame
                        .stack
                        .push(U256::from_big_endian(randao.0.as_slice()));
                    self.env.consumed_gas += gas_cost::PREVRANDAO
                }
                Opcode::GASLIMIT => {
                    if self.env.consumed_gas + gas_cost::GASLIMIT > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let gas_limit = block_env.gas_limit;
                    current_call_frame.stack.push(U256::from(gas_limit));
                    self.env.consumed_gas += gas_cost::GASLIMIT
                }
                Opcode::CHAINID => {
                    if self.env.consumed_gas + gas_cost::CHAINID > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let chain_id = block_env.chain_id;
                    current_call_frame.stack.push(U256::from(chain_id));
                    self.env.consumed_gas += gas_cost::CHAINID
                }
                Opcode::SELFBALANCE => {
                    if self.env.consumed_gas + gas_cost::SELFBALANCE > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    self.env.consumed_gas += gas_cost::SELFBALANCE;
                    todo!("when we have accounts implemented");
                }
                Opcode::BASEFEE => {
                    if self.env.consumed_gas + gas_cost::BASEFEE > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let base_fee = block_env.base_fee_per_gas;
                    current_call_frame.stack.push(base_fee);
                    self.env.consumed_gas += gas_cost::BASEFEE
                }
                Opcode::BLOBHASH => {
                    if self.env.consumed_gas + gas_cost::BLOBHASH > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    self.env.consumed_gas += gas_cost::BLOBHASH;
                    todo!("when we have tx implemented");
                }
                Opcode::BLOBBASEFEE => {
                    if self.env.consumed_gas + gas_cost::BLOBBASEFEE > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let blob_base_fee = block_env.calculate_blob_gas_price();
                    current_call_frame.stack.push(blob_base_fee);
                    self.env.consumed_gas += gas_cost::BLOBBASEFEE
                }
                Opcode::PUSH0 => {
                    if self.env.consumed_gas + gas_cost::PUSH0 > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    current_call_frame.stack.push(U256::zero());
                    self.env.consumed_gas += gas_cost::PUSH0
                }
                // PUSHn
                op if (Opcode::PUSH1..Opcode::PUSH32).contains(&op) => {
                    if self.env.consumed_gas + gas_cost::PUSHN > self.env.gas_limit {
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
                    self.env.consumed_gas += gas_cost::PUSHN
                }
                Opcode::PUSH32 => {
                    if self.env.consumed_gas + gas_cost::PUSHN > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let next_32_bytes = current_call_frame
                        .bytecode
                        .get(current_call_frame.pc()..current_call_frame.pc() + WORD_SIZE)
                        .unwrap();
                    let value_to_push = U256::from(next_32_bytes);
                    current_call_frame.stack.push(value_to_push);
                    current_call_frame.increment_pc_by(WORD_SIZE);
                    self.env.consumed_gas += gas_cost::PUSHN
                }
                Opcode::AND => {
                    if self.env.consumed_gas + gas_cost::AND > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let a = current_call_frame.stack.pop().unwrap();
                    let b = current_call_frame.stack.pop().unwrap();
                    current_call_frame.stack.push(a & b);
                    self.env.consumed_gas += gas_cost::AND
                }
                Opcode::OR => {
                    if self.env.consumed_gas + gas_cost::OR > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let a = current_call_frame.stack.pop().unwrap();
                    let b = current_call_frame.stack.pop().unwrap();
                    current_call_frame.stack.push(a | b);
                    self.env.consumed_gas += gas_cost::OR
                }
                Opcode::XOR => {
                    if self.env.consumed_gas + gas_cost::XOR > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let a = current_call_frame.stack.pop().unwrap();
                    let b = current_call_frame.stack.pop().unwrap();
                    current_call_frame.stack.push(a ^ b);
                    self.env.consumed_gas += gas_cost::XOR
                }
                Opcode::NOT => {
                    if self.env.consumed_gas + gas_cost::NOT > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let a = current_call_frame.stack.pop().unwrap();
                    current_call_frame.stack.push(!a);
                    self.env.consumed_gas += gas_cost::NOT
                }
                Opcode::BYTE => {
                    if self.env.consumed_gas + gas_cost::BYTE > self.env.gas_limit {
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
                    self.env.consumed_gas += gas_cost::BYTE
                }
                Opcode::SHL => {
                    if self.env.consumed_gas + gas_cost::SHL > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let shift = current_call_frame.stack.pop().unwrap();
                    let value = current_call_frame.stack.pop().unwrap();
                    if shift < U256::from(256) {
                        current_call_frame.stack.push(value << shift);
                    } else {
                        current_call_frame.stack.push(U256::zero());
                    }
                    self.env.consumed_gas += gas_cost::SHL
                }
                Opcode::SHR => {
                    if self.env.consumed_gas + gas_cost::SHR > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let shift = current_call_frame.stack.pop().unwrap();
                    let value = current_call_frame.stack.pop().unwrap();
                    if shift < U256::from(256) {
                        current_call_frame.stack.push(value >> shift);
                    } else {
                        current_call_frame.stack.push(U256::zero());
                    }
                    self.env.consumed_gas += gas_cost::SHR
                }
                Opcode::SAR => {
                    if self.env.consumed_gas + gas_cost::SAR > self.env.gas_limit {
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
                    self.env.consumed_gas += gas_cost::SAR
                }
                // DUPn
                op if (Opcode::DUP1..=Opcode::DUP16).contains(&op) => {
                    if self.env.consumed_gas + gas_cost::DUPN > self.env.gas_limit {
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
                    self.env.consumed_gas += gas_cost::DUPN
                }
                // SWAPn
                op if (Opcode::SWAP1..=Opcode::SWAP16).contains(&op) => {
                    if self.env.consumed_gas + gas_cost::SWAPN > self.env.gas_limit {
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
                    self.env.consumed_gas += gas_cost::SWAPN
                }
                Opcode::POP => {
                    if self.env.consumed_gas + gas_cost::POP > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    current_call_frame.stack.pop().unwrap();
                    self.env.consumed_gas += gas_cost::POP
                }
                op if (Opcode::LOG0..=Opcode::LOG4).contains(&op) => {
                    if current_call_frame.is_static {
                        panic!("Cannot create log in static context"); // should return an error and halt
                    }

                    let topic_count = (op as u8) - (Opcode::LOG0 as u8);
                    let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let size = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let topics = (0..topic_count)
                        .map(|_| {
                            let topic = current_call_frame.stack.pop().unwrap().as_u32();
                            H32::from_slice(topic.to_be_bytes().as_ref())
                        })
                        .collect();

                    let memory_expansion_cost =
                        current_call_frame.memory.expansion_cost(offset + size) as u64;
                    let gas_cost = gas_cost::LOGN_STATIC
                        + gas_cost::LOGN_DYNAMIC_BASE * topic_count as u64
                        + gas_cost::LOGN_DYNAMIC_BYTE_BASE * size as u64
                        + memory_expansion_cost;
                    if self.env.consumed_gas + gas_cost > self.env.gas_limit {
                        break; // should revert the tx
                    }

                    let data = current_call_frame.memory.load_range(offset, size);
                    let log = Log {
                        address: current_call_frame.msg_sender, // Should change the addr if we are on a Call/Create transaction (Call should be the contract we are calling, Create should be the original caller)
                        topics,
                        data: Bytes::from(data),
                    };
                    current_call_frame.logs.push(log);
                    self.env.consumed_gas += gas_cost
                }
                Opcode::MLOAD => {
                    let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let memory_expansion_cost =
                        current_call_frame.memory.expansion_cost(offset + WORD_SIZE);
                    let gas_cost = gas_cost::MLOAD_STATIC + memory_expansion_cost as u64;
                    if self.env.consumed_gas + gas_cost > self.env.gas_limit {
                        break; // should revert the tx
                    }

                    let value = current_call_frame.memory.load(offset);
                    current_call_frame.stack.push(value);
                    self.env.consumed_gas += gas_cost
                }
                Opcode::MSTORE => {
                    let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let memory_expansion_cost =
                        current_call_frame.memory.expansion_cost(offset + WORD_SIZE);
                    let gas_cost = gas_cost::MSTORE_STATIC + memory_expansion_cost as u64;
                    if self.env.consumed_gas + gas_cost > self.env.gas_limit {
                        break; // should revert the tx
                    }

                    let value = current_call_frame.stack.pop().unwrap();
                    let mut value_bytes = [0u8; WORD_SIZE];
                    value.to_big_endian(&mut value_bytes);

                    current_call_frame.memory.store_bytes(offset, &value_bytes);
                    self.env.consumed_gas += gas_cost
                }
                Opcode::MSTORE8 => {
                    let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let memory_expansion_cost =
                        current_call_frame.memory.expansion_cost(offset + 1);
                    let gas_cost = gas_cost::MSTORE8_STATIC + memory_expansion_cost as u64;
                    if self.env.consumed_gas + gas_cost > self.env.gas_limit {
                        break; // should revert the tx
                    }

                    let value = current_call_frame.stack.pop().unwrap();
                    let mut value_bytes = [0u8; WORD_SIZE];
                    value.to_big_endian(&mut value_bytes);

                    current_call_frame
                        .memory
                        .store_bytes(offset, value_bytes[WORD_SIZE - 1..WORD_SIZE].as_ref());
                    self.env.consumed_gas += gas_cost
                }
                Opcode::SLOAD => {
                    let key = current_call_frame.stack.pop().unwrap();
                    let address = if let Some(delegate) = current_call_frame.delegate {
                        delegate
                    } else {
                        current_call_frame.code_address
                    };

                    let current_value = self
                        .db
                        .read_account_storage(&address, &key)
                        .unwrap_or_default()
                        .current_value;
                    current_call_frame.stack.push(current_value);
                }
                Opcode::SSTORE => {
                    if current_call_frame.is_static {
                        panic!("Cannot write to storage in a static context");
                    }

                    let key = current_call_frame.stack.pop().unwrap();
                    let value = current_call_frame.stack.pop().unwrap();
                    // maybe we need the journal struct as accessing the Db could be slow, with the journal
                    // we can have prefetched values directly in memory and only commits the values to the db once everything is done

                    let address = if let Some(delegate) = current_call_frame.delegate {
                        delegate
                    } else {
                        current_call_frame.code_address
                    };

                    let slot = self.db.read_account_storage(&address, &key);
                    let (original_value, _) = match slot {
                        Some(slot) => (slot.original_value, slot.current_value),
                        None => (value, value),
                    };

                    self.db.write_account_storage(
                        &address,
                        key,
                        StorageSlot {
                            original_value,
                            current_value: value,
                        },
                    );
                }
                Opcode::MSIZE => {
                    if self.env.consumed_gas + gas_cost::MSIZE > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    current_call_frame
                        .stack
                        .push(current_call_frame.memory.size());
                    self.env.consumed_gas += gas_cost::MSIZE
                }
                Opcode::GAS => {
                    if self.env.consumed_gas + gas_cost::GAS > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let remaining_gas = self.env.gas_limit - self.env.consumed_gas - gas_cost::GAS;
                    current_call_frame.stack.push(remaining_gas.into());
                    self.env.consumed_gas += gas_cost::GAS
                }
                Opcode::MCOPY => {
                    let dest_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let src_offset: usize =
                        current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let size: usize = current_call_frame.stack.pop().unwrap().try_into().unwrap();

                    let words_copied = (size + WORD_SIZE - 1) / WORD_SIZE;
                    let memory_byte_size = (src_offset + size).max(dest_offset + size);
                    let memory_expansion_cost =
                        current_call_frame.memory.expansion_cost(memory_byte_size);
                    let gas_cost = gas_cost::MCOPY_STATIC
                        + gas_cost::MCOPY_DYNAMIC_BASE * words_copied as u64
                        + memory_expansion_cost as u64;

                    self.env.consumed_gas += gas_cost;
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
                    let args_offset: usize =
                        current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let args_size: usize =
                        current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let ret_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let ret_size = current_call_frame.stack.pop().unwrap().try_into().unwrap();

                    let memory_byte_size = (args_offset + args_size).max(ret_offset + ret_size);
                    let memory_expansion_cost =
                        current_call_frame.memory.expansion_cost(memory_byte_size);
                    let code_execution_cost = 0; // TODO
                    let address_access_cost =
                        if self.accrued_substate.warm_addresses.contains(&code_address) {
                            call_opcode::WARM_ADDRESS_ACCESS_COST
                        } else {
                            call_opcode::COLD_ADDRESS_ACCESS_COST
                        };
                    let positive_value_cost = if !value.is_zero() {
                        call_opcode::NON_ZERO_VALUE_COST
                            + call_opcode::BASIC_FALLBACK_FUNCTION_STIPEND
                    } else {
                        0
                    };
                    let account = self.db.accounts.get(&code_address).unwrap(); // if the account doesn't exist, it should be created
                    let value_to_empty_account_cost = if !value.is_zero() && account.is_empty() {
                        call_opcode::VALUE_TO_EMPTY_ACCOUNT_COST
                    } else {
                        0
                    };
                    // has to be returned to the caller
                    let gas_cost = memory_expansion_cost as u64
                        + code_execution_cost
                        + address_access_cost
                        + positive_value_cost
                        + value_to_empty_account_cost;
                    if self.env.consumed_gas + gas_cost > self.env.gas_limit {
                        break; // should revert the tx
                    }

                    self.accrued_substate.warm_addresses.insert(code_address);

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
                Opcode::CALLCODE => {
                    // Creates a new sub context as if calling itself, but with the code of the given account. In particular the storage remains the same. Note that an account with no code will return success as true.
                    let gas = current_call_frame.stack.pop().unwrap();
                    let code_address =
                        Address::from_low_u64_be(current_call_frame.stack.pop().unwrap().low_u64());
                    let value = current_call_frame.stack.pop().unwrap();
                    let args_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let args_size = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let ret_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let ret_size = current_call_frame.stack.pop().unwrap().try_into().unwrap();

                    let msg_sender = current_call_frame.msg_sender; // msg_sender is changed to the proxy's address
                    let to = current_call_frame.to; // to remains the same
                    let is_static = current_call_frame.is_static;

                    self.generic_call(
                        &mut current_call_frame,
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
                    );
                }
                Opcode::RETURN => {
                    let offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let size = current_call_frame.stack.pop().unwrap().try_into().unwrap();

                    let gas_cost = current_call_frame.memory.expansion_cost(offset + size) as u64;
                    if self.env.consumed_gas + gas_cost > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let return_data = current_call_frame.memory.load_range(offset, size).into();
                    if let Some(mut parent_call_frame) = self.call_frames.pop() {
                        if let (Some(_ret_offset), Some(_ret_size)) = (
                            parent_call_frame.return_data_offset,
                            parent_call_frame.return_data_size,
                        ) {
                            parent_call_frame.returndata = return_data;
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
                    self.env.consumed_gas += gas_cost;
                }
                Opcode::DELEGATECALL => {
                    // The delegatecall executes the setVars(uint256) code from Contract B but updates Contract A’s storage. The execution has the same storage, msg.sender & msg.value as its parent call setVarsDelegateCall.
                    // Creates a new sub context as if calling itself, but with the code of the given account. In particular the storage, the current sender and the current value remain the same. Note that an account with no code will return success as true.
                    let gas = current_call_frame.stack.pop().unwrap();
                    let code_address =
                        Address::from_low_u64_be(current_call_frame.stack.pop().unwrap().low_u64());
                    let args_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let args_size = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let ret_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let ret_size = current_call_frame.stack.pop().unwrap().try_into().unwrap();

                    let value = current_call_frame.msg_value; // value remains the same
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
                        Some(msg_sender),
                        false,
                        is_static,
                        args_offset,
                        args_size,
                        ret_offset,
                        ret_size,
                    );
                }
                Opcode::STATICCALL => {
                    // it cannot be used to transfer Ether
                    let gas = current_call_frame.stack.pop().unwrap();
                    let code_address =
                        Address::from_low_u64_be(current_call_frame.stack.pop().unwrap().low_u64());
                    let args_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let args_size = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let ret_offset = current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let ret_size = current_call_frame.stack.pop().unwrap().try_into().unwrap();

                    let msg_sender = current_call_frame.msg_sender; // caller remains the msg_sender
                    let value = current_call_frame.msg_value;

                    self.generic_call(
                        &mut current_call_frame,
                        gas,
                        value, // check
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
                    );
                }
                Opcode::CREATE => {
                    let value_in_wei_to_send = current_call_frame.stack.pop().unwrap();
                    let code_offset_in_memory =
                        current_call_frame.stack.pop().unwrap().try_into().unwrap();
                    let code_size_in_memory =
                        current_call_frame.stack.pop().unwrap().try_into().unwrap();

                    self.create(
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

                    self.create(
                        value_in_wei_to_send,
                        code_offset_in_memory,
                        code_size_in_memory,
                        Some(salt),
                        &mut current_call_frame,
                    );
                }
                Opcode::TLOAD => {
                    if self.env.consumed_gas + gas_cost::TLOAD > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let key = current_call_frame.stack.pop().unwrap();
                    let value = current_call_frame
                        .transient_storage
                        .get(&(current_call_frame.msg_sender, key))
                        .cloned()
                        .unwrap_or(U256::zero());

                    current_call_frame.stack.push(value);
                    self.env.consumed_gas += gas_cost::TLOAD
                }
                Opcode::TSTORE => {
                    if self.env.consumed_gas + gas_cost::TSTORE > self.env.gas_limit {
                        break; // should revert the tx
                    }
                    let key = current_call_frame.stack.pop().unwrap();
                    let value = current_call_frame.stack.pop().unwrap();

                    current_call_frame
                        .transient_storage
                        .insert((current_call_frame.msg_sender, key), value);
                    self.env.consumed_gas += gas_cost::TSTORE
                }
                _ => unimplemented!(),
            }
        }
        // self.consumed_gas = tx_env.consumed_gas;
        self.call_frames.push(current_call_frame);
    }

    pub fn current_call_frame_mut(&mut self) -> &mut CallFrame {
        self.call_frames.last_mut().unwrap()
    }

    pub fn current_call_frame(&self) -> &CallFrame {
        self.call_frames.last().unwrap()
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
    ) {
        // check balance
        if self.db.balance(&current_call_frame.msg_sender) < value {
            current_call_frame.stack.push(U256::from(REVERT_FOR_CALL));
            return;
        }

        // transfer value
        // transfer(&current_call_frame.msg_sender, &address, value);

        let callee_bytecode = self.db.get_account_bytecode(&code_address);

        if callee_bytecode.is_empty() {
            current_call_frame.stack.push(U256::from(SUCCESS_FOR_CALL));
            return;
        }

        let calldata = current_call_frame
            .memory
            .load_range(args_offset, args_size)
            .into();

        let new_call_frame = CallFrame::new(
            gas,
            msg_sender,
            to,
            code_address,
            delegate,
            callee_bytecode,
            value,
            calldata,
            is_static,
        );

        current_call_frame.return_data_offset = Some(ret_offset);
        current_call_frame.return_data_size = Some(ret_size);

        self.call_frames.push(current_call_frame.clone());
        *current_call_frame = new_call_frame;
    }

    /// Calculates the address of a new conctract using the CREATE opcode as follow
    ///
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
    ///
    /// initialization_code = memory[offset:offset+size]
    ///
    /// address = keccak256(0xff + sender_address + salt + keccak256(initialization_code))[12:]
    pub fn calculate_create2_address(
        sender_address: Address,
        initialization_code: &Bytes,
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

    /// Common behavior for CREATE and CREATE2 opcodes
    ///
    /// Could be used for CREATE type transactions
    pub fn create(
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
            .db
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
        sender_account.nonce = new_nonce;
        sender_account.balance -= value_in_wei_to_send;
        let code = Bytes::from(
            current_call_frame
                .memory
                .load_range(code_offset_in_memory, code_size_in_memory),
        );

        let new_address = match salt {
            Some(salt) => {
                Self::calculate_create2_address(current_call_frame.msg_sender, &code, salt)
            }
            None => {
                Self::calculate_create_address(current_call_frame.msg_sender, sender_account.nonce)
            }
        };

        if self.db.accounts.contains_key(&new_address) {
            current_call_frame.stack.push(U256::from(REVERT_FOR_CREATE));
            return;
        }
        // nonce == 1, as we will execute this contract right now
        let new_account = Account::new(value_in_wei_to_send, code.clone(), 1, Default::default());
        self.db.add_account(new_address, new_account);

        let mut gas = current_call_frame.gas;
        gas -= gas / 64; // 63/64 of the gas to the call
        current_call_frame.gas -= gas; // leaves 1/64  of the gas to current call frame

        let new_call_frame = CallFrame::new(
            gas,
            current_call_frame.msg_sender,
            new_address,
            new_address,
            None,
            code,
            value_in_wei_to_send,
            Bytes::new(),
            false,
        );

        current_call_frame.return_data_offset = Some(code_offset_in_memory);
        current_call_frame.return_data_size = Some(code_size_in_memory);

        current_call_frame.stack.push(address_to_word(new_address));

        self.call_frames.push(current_call_frame.clone());
        *current_call_frame = new_call_frame;
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
