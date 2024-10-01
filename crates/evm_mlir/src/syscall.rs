//! # Module implementing syscalls for the EVM
//!
//! The syscalls implemented here are to be exposed to the generated code
//! via [`register_syscalls`]. Each syscall implements functionality that's
//! not possible to implement in the generated code, such as interacting with
//! the storage, or just difficult, like allocating memory in the heap
//! ([`SyscallContext::extend_memory`]).
//!
//! ### Adding a new syscall
//!
//! New syscalls should be implemented by adding a new method to the [`SyscallContext`]
//! struct (see [`SyscallContext::write_result`] for an example). After that, the syscall
//! should be registered in the [`register_syscalls`] function, which will make it available
//! to the generated code. Afterwards, the syscall should be declared in
//! [`mlir::declare_syscalls`], which will make the syscall available inside the MLIR code.
//! Finally, the function can be called from the MLIR code like a normal function (see
//! [`mlir::write_result_syscall`] for an example).
use std::ffi::c_void;

use crate::{
    constants::{
        call_opcode::{self},
        gas_cost::{self, MAX_CODE_SIZE},
        return_codes, CallType,
    },
    context::Context,
    db::AccountInfo,
    env::{Env, TransactTo},
    executor::{Executor, OptLevel},
    journal::Journal,
    precompiles::*,
    primitives::{Address, Bytes, B256, U256 as EU256},
    program::Program,
    result::{EVMError, ExecutionResult, HaltReason, Output, ResultAndState, SuccessReason},
    state::AccountStatus,
    utils::{compute_contract_address, compute_contract_address2},
};
use melior::ExecutionEngine;
use sha3::{Digest, Keccak256};
use std::collections::HashMap;

/// Function type for the main entrypoint of the generated code
pub type MainFunc = extern "C" fn(&mut SyscallContext, initial_gas: u64) -> u8;

pub const GAS_REFUND_DENOMINATOR: u64 = 5;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(C, align(16))]
pub struct U256 {
    pub lo: u128,
    pub hi: u128,
}

impl U256 {
    pub fn from_fixed_be_bytes(bytes: [u8; 32]) -> Self {
        let hi = u128::from_be_bytes(bytes[0..16].try_into().unwrap());
        let lo = u128::from_be_bytes(bytes[16..32].try_into().unwrap());
        U256 { hi, lo }
    }

    pub fn copy_from(&mut self, value: &Address) {
        let mut buffer = [0u8; 32];
        buffer[12..32].copy_from_slice(&value.0);
        self.lo = u128::from_be_bytes(buffer[16..32].try_into().unwrap());
        self.hi = u128::from_be_bytes(buffer[0..16].try_into().unwrap());
    }

    pub fn to_primitive_u256(&self) -> EU256 {
        (EU256::from(self.hi) << 128) + self.lo
    }

    pub fn zero() -> U256 {
        U256 { lo: 0, hi: 0 }
    }
}

impl From<&U256> for Address {
    fn from(value: &U256) -> Self {
        // NOTE: return an address using the last 20 bytes, discarding the first 12 bytes.
        let hi_bytes = value.hi.to_be_bytes();
        let lo_bytes = value.lo.to_be_bytes();
        let address = [&hi_bytes[12..16], &lo_bytes[..]].concat();
        Address::from_slice(&address)
    }
}

#[derive(Debug, Clone)]
pub enum ExitStatusCode {
    Return = 0,
    Stop,
    Revert,
    Error,
    Default,
}
impl ExitStatusCode {
    #[inline(always)]
    pub fn to_u8(self) -> u8 {
        self as u8
    }
    pub fn from_u8(value: u8) -> Self {
        match value {
            x if x == Self::Return.to_u8() => Self::Return,
            x if x == Self::Stop.to_u8() => Self::Stop,
            x if x == Self::Revert.to_u8() => Self::Revert,
            x if x == Self::Error.to_u8() => Self::Error,
            _ => Self::Default,
        }
    }
}

#[derive(Debug, Default)]
pub struct InnerContext {
    /// The memory segment of the EVM.
    /// For extending it, see [`Self::extend_memory`]
    pub memory: Vec<u8>,
    /// The result of the execution
    return_data: Option<(usize, usize)>,
    // The program bytecode
    pub program: Vec<u8>,
    pub gas_remaining: Option<u64>,
    gas_refund: u64,
    exit_status: Option<ExitStatusCode>,
    logs: Vec<LogData>,
}

impl InnerContext {
    pub fn get_memory_slice(&mut self, memory_offset: usize, size: usize) -> &mut [u8] {
        let memory_end = memory_offset + size;

        &mut self.memory[memory_offset..memory_end]
    }

    pub fn resize_memory_if_necessary(&mut self, offset: usize, data_size: usize) {
        if offset + data_size > self.memory.len() {
            self.memory.resize(offset + data_size, 0);
        }
    }

    pub fn set_value_to_memory(
        &mut self,
        memory_offset: usize,
        value_offset: usize,
        size: usize,
        value: &[u8],
    ) {
        if value_offset >= value.len() {
            self.get_memory_slice(memory_offset, size).fill(0);
            return;
        }

        let value_memory_end = value.len().min(value_offset + size);
        let value_memory_size = value_memory_end - value_offset;
        debug_assert!(value_offset < value.len() && value_memory_end <= value.len());
        let value = &value[value_offset..value_memory_end];
        self.get_memory_slice(memory_offset, value_memory_size)
            .copy_from_slice(value);

        self.get_memory_slice(memory_offset + value_memory_size, size - value_memory_size)
            .fill(0);
    }
}

/// Information about current call frame
#[derive(Debug, Default)]
pub struct CallFrame {
    pub caller: Address,
    ctx_is_static: bool,
    last_call_return_data: Vec<u8>,
}

impl CallFrame {
    pub fn new(caller: Address) -> Self {
        Self {
            caller,
            ctx_is_static: false,
            ..Default::default()
        }
    }
}

/// The context passed to syscalls
#[derive(Debug)]
pub struct SyscallContext<'c> {
    pub env: Env,
    pub journal: Journal<'c>,
    pub call_frame: CallFrame,
    pub inner_context: InnerContext,
    pub halt_reason: Option<HaltReason>,
    initial_gas: u64,
    pub transient_storage: HashMap<(Address, EU256), EU256>, // TODO: Move this to Journal
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct LogData {
    pub topics: Vec<U256>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
pub struct Log {
    pub address: Address,
    pub data: LogData,
}

/// Accessors for disponibilizing the execution results
impl<'c> SyscallContext<'c> {
    pub fn new(env: Env, journal: Journal<'c>, call_frame: CallFrame, initial_gas: u64) -> Self {
        Self {
            env,
            initial_gas,
            journal,
            call_frame,
            halt_reason: None,
            inner_context: Default::default(),
            transient_storage: Default::default(),
        }
    }

    pub fn return_values(&self) -> &[u8] {
        let (offset, size) = self.inner_context.return_data.unwrap_or((0, 0));
        if offset + size > self.inner_context.memory.len() {
            return &[];
        }
        &self.inner_context.memory[offset..offset + size]
    }

    pub fn logs(&self) -> Vec<Log> {
        self.inner_context
            .logs
            .iter()
            .map(|logdata| Log {
                address: self.env.tx.get_address(),
                data: logdata.clone(),
            })
            .collect()
    }

    pub fn get_result(&self) -> Result<ResultAndState, EVMError> {
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
}

/// Syscall implementations
///
/// Note that each function is marked as `extern "C"`, which is necessary for the
/// function to be callable from the generated code.
impl<'c> SyscallContext<'c> {
    pub extern "C" fn write_result(
        &mut self,
        offset: u32,
        bytes_len: u32,
        remaining_gas: u64,
        execution_result: u8,
    ) {
        self.inner_context.return_data = Some((offset as usize, bytes_len as usize));
        self.inner_context.gas_remaining = Some(remaining_gas);
        self.inner_context.exit_status = Some(ExitStatusCode::from_u8(execution_result));
    }

    pub extern "C" fn get_return_data_size(&mut self) -> u32 {
        self.call_frame.last_call_return_data.len() as _
    }

    pub extern "C" fn copy_return_data_into_memory(
        &mut self,
        dest_offset: u32,
        offset: u32,
        size: u32,
    ) {
        self.inner_context
            .resize_memory_if_necessary(dest_offset as usize, size as usize);

        self.inner_context.set_value_to_memory(
            dest_offset as usize,
            offset as usize,
            size as usize,
            &self.call_frame.last_call_return_data,
        );
    }

    pub extern "C" fn call(
        &mut self,
        mut gas_to_send: u64,
        call_to_address: &U256,
        value_to_transfer: &U256,
        args_offset: u32,
        args_size: u32,
        ret_offset: u32,
        ret_size: u32,
        available_gas: u64,
        consumed_gas: &mut u64,
        call_type: u8,
    ) -> u8 {
        //TODO: Add call depth check
        //TODO: Check that the args offsets and sizes are correct -> This from the MLIR side
        //TODO: This should instead add the account fetch (warm or cold) cost
        //For the moment we consider warm access
        let callee_address = Address::from(call_to_address);
        //Copy the calldata from memory
        let off = args_offset as usize;
        let size = args_size as usize;

        if off + size > self.inner_context.memory.len() {
            eprintln!("ERROR: size + offset too big");
            self.halt_reason = Some(HaltReason::OutOfOffset);
            return return_codes::HALT_RETURN_CODE;
        }

        let calldata = Bytes::copy_from_slice(&self.inner_context.memory[off..off + size]);

        *consumed_gas = gas_cost::CALL_WARM as u64;

        let (return_code, return_data) = if is_precompile(callee_address) {
            execute_precompile(callee_address, calldata, gas_to_send, consumed_gas)
        } else {
            // Execute subcontext
            //TODO: Add call depth check
            //TODO: Check that the args offsets and sizes are correct -> This from the MLIR side
            let value = value_to_transfer.to_primitive_u256();
            let call_type = CallType::try_from(call_type)
                .expect("Error while parsing CallType on call syscall");

            let callee_account = match self.journal.get_account(&callee_address) {
                Some(account) => {
                    let is_cold = !self.journal.account_is_warm(&callee_address);

                    if is_cold {
                        *consumed_gas = gas_cost::CALL_COLD as u64;
                        self.journal.add_account_as_warm(callee_address);
                    } else {
                        *consumed_gas = gas_cost::CALL_WARM as u64;
                    }

                    account
                }
                None => {
                    *consumed_gas = 0;
                    return return_codes::REVERT_RETURN_CODE;
                }
            };

            let caller_address = self.env.tx.get_address();
            let caller_account = self
                .journal
                .get_account(&caller_address)
                .unwrap_or_default();

            let mut stipend = 0;
            if !value.is_zero() {
                if caller_account.balance < value {
                    //There isn't enough balance to send
                    return return_codes::REVERT_RETURN_CODE;
                }
                *consumed_gas += call_opcode::NOT_ZERO_VALUE_COST;
                if callee_account.is_empty() {
                    *consumed_gas += call_opcode::EMPTY_CALLEE_COST;
                }
                if available_gas < *consumed_gas {
                    self.halt_reason =
                        Some(HaltReason::OutOfGas(crate::result::OutOfGasError::Basic));
                    return return_codes::HALT_RETURN_CODE; //It acctually doesn't matter what we return here
                }
                stipend = call_opcode::STIPEND_GAS_ADDITION;

                //TODO: Maybe we should increment the nonce too
                let caller_balance = caller_account.balance;
                self.journal
                    .set_balance(&caller_address, caller_balance - value);

                let callee_balance = callee_account.balance;
                self.journal
                    .set_balance(&callee_address, callee_balance + value);
            }

            let remaining_gas = available_gas.saturating_sub(*consumed_gas);
            gas_to_send = std::cmp::min(
                remaining_gas / call_opcode::GAS_CAP_DIVISION_FACTOR,
                gas_to_send,
            );
            *consumed_gas += gas_to_send;
            gas_to_send += stipend;

            let mut env = self.env.clone();

            //TODO: Check if calling `get_address()` here is ok
            let this_address = self.env.tx.get_address();
            let (new_frame_caller, new_value, transact_to) = match call_type {
                CallType::Call | CallType::StaticCall => (this_address, value, callee_address),
                CallType::CallCode => (this_address, value, this_address),
                CallType::DelegateCall => (self.call_frame.caller, self.env.tx.value, this_address),
            };

            env.tx.value = new_value;
            env.tx.transact_to = TransactTo::Call(transact_to);
            env.tx.gas_limit = gas_to_send;

            //Copy the calldata from memory
            let off = args_offset as usize;
            let size = args_size as usize;
            env.tx.data = Bytes::from(self.inner_context.memory[off..off + size].to_vec());

            //NOTE: We could optimize this by not making the call if the bytecode is zero.
            //We would have to refund the stipend here
            //TODO: Check if returning REVERT because of database fail is ok
            let bytecode = self.journal.code_by_address(&callee_address);

            let program = Program::from_bytecode(&bytecode);

            let context = Context::new();
            let module = context
                .compile(&program, Default::default())
                .expect("failed to compile program");

            let is_static = self.call_frame.ctx_is_static || call_type == CallType::StaticCall;

            let call_frame = CallFrame {
                caller: new_frame_caller,
                ctx_is_static: is_static,
                ..Default::default()
            };

            let journal = self.journal.eject_base();

            let mut context = SyscallContext::new(env.clone(), journal, call_frame, gas_to_send);
            let executor = Executor::new(&module, &context, OptLevel::Aggressive);

            executor.execute(&mut context, env.tx.gas_limit);

            let result = context.get_result().unwrap().result;

            let unused_gas = gas_to_send - result.gas_used();
            *consumed_gas -= unused_gas;
            *consumed_gas -= result.gas_refunded();
            let return_code = if result.is_success() {
                self.journal.extend_from_successful(context.journal);
                return_codes::SUCCESS_RETURN_CODE
            } else {
                //TODO: If we revert, should we still send the value to the called contract?
                self.journal.extend_from_reverted(context.journal);
                return_codes::REVERT_RETURN_CODE
            };
            let output = result.into_output().unwrap_or_default();
            (return_code, output)
        };

        //TODO: This copying mechanism may be improved with a safe copy_from_slice which would
        //reduce the need of calling return_data.to_vec()
        self.call_frame.last_call_return_data.clear();
        self.call_frame
            .last_call_return_data
            .clone_from(&return_data.to_vec());

        if return_code == return_codes::SUCCESS_RETURN_CODE {
            self.inner_context
                .resize_memory_if_necessary(ret_offset as usize, ret_size as usize);

            self.inner_context.set_value_to_memory(
                ret_offset as usize,
                0,
                ret_size as usize,
                &return_data,
            );
        }
        return_code
    }

    pub extern "C" fn store_in_selfbalance_ptr(&mut self, balance: &mut U256) {
        let account = match self.env.tx.transact_to {
            TransactTo::Call(address) => self.journal.get_account(&address).unwrap_or_default(),
            TransactTo::Create => AccountInfo::default(), //This branch should never happen
        };
        balance.hi = (account.balance >> 128).low_u128();
        balance.lo = account.balance.low_u128();
    }

    pub extern "C" fn keccak256_hasher(&mut self, offset: u32, size: u32, hash_ptr: &mut U256) {
        let offset = offset as usize;
        let size = size as usize;
        self.inner_context.resize_memory_if_necessary(offset, size);
        let data = &self.inner_context.memory[offset..offset + size];
        let mut hasher = Keccak256::new();
        hasher.update(data);
        let result = hasher.finalize();
        *hash_ptr = U256::from_fixed_be_bytes(result.into());
    }

    pub extern "C" fn store_in_callvalue_ptr(&self, value: &mut U256) {
        let aux = &self.env.tx.value;
        value.lo = aux.low_u128();
        value.hi = (aux >> 128).low_u128();
    }

    pub extern "C" fn store_in_blobbasefee_ptr(&self, value: &mut u128) {
        *value = self.env.block.blob_gasprice.unwrap_or_default();
    }

    pub extern "C" fn get_gaslimit(&self) -> u64 {
        self.env.tx.gas_limit
    }

    pub extern "C" fn store_in_caller_ptr(&self, value: &mut U256) {
        value.copy_from(&self.call_frame.caller);
    }

    pub extern "C" fn store_in_gasprice_ptr(&self, value: &mut U256) {
        let aux = &self.env.tx.gas_price;
        value.lo = aux.low_u128();
        value.hi = (aux >> 128).low_u128();
    }

    pub extern "C" fn get_chainid(&self) -> u64 {
        self.env.cfg.chain_id
    }

    pub extern "C" fn get_calldata_ptr(&mut self) -> *const u8 {
        self.env.tx.data.as_ptr()
    }

    pub extern "C" fn get_calldata_size_syscall(&self) -> u32 {
        self.env.tx.data.len() as u32
    }

    pub extern "C" fn get_origin(&self, address: &mut U256) {
        let aux = &self.env.tx.caller;
        address.copy_from(aux);
    }

    pub extern "C" fn extend_memory(&mut self, new_size: u32) -> *mut u8 {
        let new_size = new_size as usize;
        if new_size <= self.inner_context.memory.len() {
            return self.inner_context.memory.as_mut_ptr();
        }
        match self
            .inner_context
            .memory
            .try_reserve(new_size - self.inner_context.memory.len())
        {
            Ok(()) => {
                self.inner_context.memory.resize(new_size, 0);
                self.inner_context.memory.as_mut_ptr()
            }
            // TODO: use tracing here
            Err(err) => {
                eprintln!("Failed to reserve memory: {err}");
                std::ptr::null_mut()
            }
        }
    }

    pub extern "C" fn copy_code_to_memory(
        &mut self,
        code_offset: u32,
        size: u32,
        dest_offset: u32,
    ) {
        let code_size = self.inner_context.program.len();
        // cast everything to `usize`
        let code_offset = code_offset as usize;
        let size = size as usize;
        let dest_offset = dest_offset as usize;

        self.inner_context
            .resize_memory_if_necessary(dest_offset, size);

        let code = &self.inner_context.program.clone()[..code_size];
        self.inner_context
            .set_value_to_memory(dest_offset, code_offset, size, code);
    }

    pub extern "C" fn read_storage(&mut self, stg_key: &U256, stg_value: &mut U256) -> i64 {
        let address = self.env.tx.get_address();

        let key = stg_key.to_primitive_u256();
        let is_cold = !self.journal.key_is_warm(&address, &key);
        // Read value from journaled_storage. If there isn't one, then read from db
        let result = self
            .journal
            .read_storage(&address, &key)
            .unwrap_or_default()
            .present_value;

        stg_value.hi = (result >> 128).low_u128();
        stg_value.lo = result.low_u128();

        if is_cold {
            gas_cost::SLOAD_COLD
        } else {
            gas_cost::SLOAD_WARM
        }
    }

    pub extern "C" fn write_storage(&mut self, stg_key: &U256, stg_value: &mut U256) -> i64 {
        let key = stg_key.to_primitive_u256();
        let value = stg_value.to_primitive_u256();
        // TODO: Check if this case is ok. Can storage be written on Create?
        let TransactTo::Call(address) = self.env.tx.transact_to else {
            return 0;
        };

        let is_cold = !self.journal.key_is_warm(&address, &key);
        let slot = self.journal.read_storage(&address, &key);
        self.journal.write_storage(&address, key, value);

        let (original, current) = match slot {
            Some(slot) => (slot.original_value, slot.present_value),
            None => (value, value),
        };

        // Compute the gas cost
        let mut gas_cost: i64 = if original.is_zero() && current.is_zero() && current != value {
            20_000
        } else if original == current && current != value {
            2_900
        } else {
            100
        };

        // When the value is cold, add extra 2100 gas
        if is_cold {
            gas_cost += 2_100;
        }

        // Compute the gas refund
        let reset_non_zero_to_zero = !original.is_zero() && !current.is_zero() && value.is_zero();
        let undo_reset_to_zero = !original.is_zero() && current.is_zero() && !value.is_zero();
        let undo_reset_to_zero_into_original = undo_reset_to_zero && (value == original);
        let reset_back_to_zero = original.is_zero() && !current.is_zero() && value.is_zero();
        let reset_to_original = (current != value) && (original == value);

        let gas_refund: i64 = if reset_non_zero_to_zero {
            4_800
        } else if undo_reset_to_zero_into_original {
            -2_000
        } else if undo_reset_to_zero {
            -4_800
        } else if reset_back_to_zero {
            19_900
        } else if reset_to_original {
            2_800
        } else {
            0
        };

        if gas_refund > 0 {
            self.inner_context.gas_refund += gas_refund as u64;
        } else {
            self.inner_context.gas_refund = self
                .inner_context
                .gas_refund
                .saturating_sub(gas_refund.unsigned_abs());
        };

        gas_cost
    }

    pub extern "C" fn append_log(&mut self, offset: u32, size: u32) {
        self.create_log(offset, size, vec![]);
    }

    pub extern "C" fn append_log_with_one_topic(&mut self, offset: u32, size: u32, topic: &U256) {
        self.create_log(offset, size, vec![*topic]);
    }

    pub extern "C" fn append_log_with_two_topics(
        &mut self,
        offset: u32,
        size: u32,
        topic1: &U256,
        topic2: &U256,
    ) {
        self.create_log(offset, size, vec![*topic1, *topic2]);
    }

    pub extern "C" fn append_log_with_three_topics(
        &mut self,
        offset: u32,
        size: u32,
        topic1: &U256,
        topic2: &U256,
        topic3: &U256,
    ) {
        self.create_log(offset, size, vec![*topic1, *topic2, *topic3]);
    }

    pub extern "C" fn append_log_with_four_topics(
        &mut self,
        offset: u32,
        size: u32,
        topic1: &U256,
        topic2: &U256,
        topic3: &U256,
        topic4: &U256,
    ) {
        self.create_log(offset, size, vec![*topic1, *topic2, *topic3, *topic4]);
    }

    pub extern "C" fn get_block_number(&self, number: &mut U256) {
        let block_number = self.env.block.number;

        number.hi = (block_number >> 128).low_u128();
        number.lo = block_number.low_u128();
    }

    pub extern "C" fn get_block_hash(&mut self, number: &mut U256) {
        let number_as_u256 = number.to_primitive_u256();

        // If number is not in the valid range (last 256 blocks), return zero.
        let hash = if number_as_u256 < self.env.block.number.saturating_sub(EU256::from(256))
            || number_as_u256 >= self.env.block.number
        {
            // TODO: check if this is necessary. Db should only contain last 256 blocks, so number check would not be needed.
            B256::zero()
        } else {
            self.journal.get_block_hash(&number_as_u256)
        };

        let (hi, lo) = hash.as_bytes().split_at(16);
        number.lo = u128::from_be_bytes(lo.try_into().unwrap());
        number.hi = u128::from_be_bytes(hi.try_into().unwrap());
    }

    /// Receives a memory offset and size, and a vector of topics.
    /// Creates a Log with topics and data equal to memory[offset..offset + size]
    /// and pushes it to the logs vector.
    fn create_log(&mut self, offset: u32, size: u32, topics: Vec<U256>) {
        let offset = offset as usize;
        let size = size as usize;

        self.inner_context.resize_memory_if_necessary(offset, size);
        let data: Vec<u8> = self.inner_context.memory[offset..offset + size].into();

        let log = LogData { data, topics };
        self.inner_context.logs.push(log);
    }

    pub extern "C" fn get_codesize_from_address(&mut self, address: &U256, gas: &mut u64) -> u64 {
        //TODO: Here we are returning 0 if a Database error occurs. Check this
        let is_cold = !self.journal.account_is_warm(&Address::from(address));
        let codesize = self.journal.code_by_address(&Address::from(address)).len();
        if is_cold {
            self.journal.add_account_as_warm(Address::from(address));
            *gas = gas_cost::EXTCODESIZE_COLD as u64;
        } else {
            *gas = gas_cost::EXTCODESIZE_WARM as u64;
        }

        codesize as u64
    }

    pub extern "C" fn get_address_ptr(&mut self) -> *const u8 {
        self.env.tx.get_address().to_fixed_bytes().as_ptr()
    }

    pub extern "C" fn get_prevrandao(&self, prevrandao: &mut U256) {
        let randao = self.env.block.prevrandao.unwrap_or_default();
        *prevrandao = U256::from_fixed_be_bytes(randao.into());
    }

    pub extern "C" fn get_coinbase_ptr(&self) -> *const u8 {
        self.env.block.coinbase.as_ptr()
    }

    pub extern "C" fn store_in_timestamp_ptr(&self, value: &mut U256) {
        let aux = &self.env.block.timestamp;
        value.lo = aux.low_u128();
        value.hi = (aux >> 128).low_u128();
    }

    pub extern "C" fn store_in_basefee_ptr(&self, basefee: &mut U256) {
        basefee.hi = (self.env.block.basefee >> 128).low_u128();
        basefee.lo = self.env.block.basefee.low_u128();
    }

    pub extern "C" fn store_in_balance(&mut self, address: &U256, balance: &mut U256) -> i64 {
        let mut gas_cost = gas_cost::BALANCE_COLD;

        // addresses longer than 20 bytes should be invalid
        if (address.hi >> 32) != 0 {
            balance.hi = 0;
            balance.lo = 0;
        } else {
            let address_hi_slice = address.hi.to_be_bytes();
            let address_lo_slice = address.lo.to_be_bytes();

            let address_slice = [&address_hi_slice[12..16], &address_lo_slice[..]].concat();

            let address = Address::from_slice(&address_slice);
            let is_cold = !self.journal.account_is_warm(&address);

            match self.journal.get_account(&address) {
                Some(a) => {
                    balance.hi = (a.balance >> 128).low_u128();
                    balance.lo = a.balance.low_u128();
                }
                None => {
                    balance.hi = 0;
                    balance.lo = 0;
                }
            };
            if is_cold {
                self.journal.add_account_as_warm(address);
                gas_cost = gas_cost::BALANCE_COLD
            } else {
                gas_cost = gas_cost::BALANCE_WARM
            };
        }
        gas_cost
    }

    pub extern "C" fn get_blob_hash_at_index(&mut self, index: &U256, blobhash: &mut U256) {
        if index.hi != 0 {
            *blobhash = U256::default();
            return;
        }
        *blobhash = usize::try_from(index.lo)
            .ok()
            .and_then(|idx| self.env.tx.blob_hashes.get(idx).cloned())
            .map(|x| U256::from_fixed_be_bytes(x.into()))
            .unwrap_or_default();
    }

    pub extern "C" fn copy_ext_code_to_memory(
        &mut self,
        address_value: &U256,
        code_offset: u32,
        size: u32,
        dest_offset: u32,
    ) -> u64 {
        let size = size as usize;
        let code_offset = code_offset as usize;
        let dest_offset = dest_offset as usize;
        let address = Address::from(address_value);
        // TODO: Check if returning default bytecode on database failure is ok
        // A silenced error like this may produce unexpected code behaviour
        // ----> If the code is not found, it should be a fatal error
        let code = self.journal.code_by_address(&address);

        let code_offset = code_offset.min(code.len());
        self.inner_context
            .resize_memory_if_necessary(dest_offset, size);
        self.inner_context
            .set_value_to_memory(dest_offset, code_offset, size, &code);

        let is_cold = !self.journal.account_is_warm(&address);
        if is_cold {
            self.journal.add_account_as_warm(Address::from(address));
            gas_cost::EXTCODECOPY_COLD as u64
        } else {
            gas_cost::EXTCODECOPY_WARM as u64
        }
    }

    pub extern "C" fn get_code_hash(&mut self, address: &mut U256) -> u64 {
        let is_cold = self
            .journal
            .account_is_warm(&Address::from(address as &U256));

        let gas_cost = if is_cold {
            gas_cost::EXTCODEHASH_COLD
        } else {
            gas_cost::EXTCODEHASH_WARM
        };

        let hash = match self.journal.get_account(&Address::from(address as &U256)) {
            Some(account_info) => account_info.code_hash,
            _ => {
                self.journal
                    .add_account_as_warm(Address::from(address as &U256));
                B256::zero()
            }
        };

        *address = U256::from_fixed_be_bytes(hash.to_fixed_bytes());
        gas_cost as u64
    }

    fn create_aux(
        &mut self,
        size: u32,
        offset: u32,
        value: &mut U256,
        remaining_gas: &mut u64,
        salt: Option<&U256>,
    ) -> u8 {
        let value_as_u256 = value.to_primitive_u256();
        let offset = offset as usize;
        let size = size as usize;
        let minimum_word_size = ((size + 31) / 32) as u64;
        let sender_address = self.env.tx.get_address();

        if size > MAX_CODE_SIZE * 2 {
            self.halt_reason = Some(HaltReason::CreateContractSizeLimit);
            return return_codes::HALT_RETURN_CODE;
        }

        self.inner_context.resize_memory_if_necessary(offset, size);

        let initialization_bytecode = &self.inner_context.memory[offset..offset + size];
        let program = Program::from_bytecode(initialization_bytecode);

        // if we do a create from a program, and the created program would be the same, that means a recursive create
        // and we should directly halt to avoid an stack overflow
        if self.inner_context.program == program.clone().to_bytecode() {
            self.halt_reason = Some(HaltReason::OutOfGas(
                crate::result::OutOfGasError::RecursiveCreate,
            ));
            return return_codes::HALT_RETURN_CODE;
        }

        let sender_account = self.journal.get_account(&sender_address).unwrap();

        let (dest_addr, hash_cost) = match salt {
            Some(s) => (
                compute_contract_address2(
                    sender_address,
                    s.to_primitive_u256(),
                    initialization_bytecode,
                ),
                minimum_word_size * gas_cost::HASH_WORD_COST as u64,
            ),
            _ => {
                if sender_account.nonce.checked_add(1).is_none() {
                    self.halt_reason = Some(HaltReason::NonceOverflow);
                    return return_codes::HALT_RETURN_CODE;
                }

                (
                    compute_contract_address(sender_address, sender_account.nonce),
                    0,
                )
            }
        };

        // Check if there is already a contract stored in dest_address
        if self.journal.get_account(&dest_addr).is_some() {
            self.halt_reason = Some(HaltReason::CreateCollision);
            return return_codes::HALT_RETURN_CODE;
        }
        self.journal.add_account_as_warm(dest_addr);

        // Create subcontext for the initialization code
        // TODO: Add call depth check
        let mut new_env = self.env.clone();
        new_env.tx.transact_to = TransactTo::Call(dest_addr);
        new_env.tx.gas_limit = *remaining_gas;
        let call_frame = CallFrame::new(sender_address);

        // Execute initialization code
        let context = Context::new();
        let module = context
            .compile(&program, Default::default())
            .expect("failed to compile program");

        // NOTE: Here we are not taking into account what happens if the deployment code reverts
        let ctx_journal = self.journal.eject_base();
        let mut context =
            SyscallContext::new(new_env.clone(), ctx_journal, call_frame, *remaining_gas);
        context.journal.new_account(dest_addr, value_as_u256);
        let executor = Executor::new(&module, &context, OptLevel::Aggressive);
        context.inner_context.program = program.to_bytecode();
        let program_len = context.inner_context.program.len() as u32;
        executor.execute(&mut context, new_env.tx.gas_limit);

        let result = context.get_result().unwrap().result;
        let bytecode = result.output().cloned().unwrap_or_default();

        self.journal.extend_from_successful(context.journal);

        // Set the gas cost
        let init_code_cost = minimum_word_size * gas_cost::INIT_WORD_COST as u64;
        let code_deposit_cost = (bytecode.len() as u64) * gas_cost::BYTE_DEPOSIT_COST as u64;
        let gas_cost = init_code_cost + code_deposit_cost + hash_cost + result.gas_used()
            - result.gas_refunded();
        *remaining_gas = gas_cost;

        // Check if balance is enough
        let Some(sender_balance) = sender_account.balance.checked_sub(value_as_u256) else {
            *value = U256::zero();
            self.write_result(
                offset as u32,
                program_len,
                *remaining_gas,
                return_codes::SUCCESS_RETURN_CODE,
            );
            return return_codes::SUCCESS_RETURN_CODE;
        };

        // Create new contract and update sender account
        self.journal
            .new_contract(dest_addr, bytecode, value_as_u256);

        let Some(new_nonce) = sender_account.nonce.checked_add(1) else {
            self.halt_reason = Some(HaltReason::NonceOverflow);
            return return_codes::HALT_RETURN_CODE;
        };

        self.journal.set_nonce(&sender_address, new_nonce);
        self.journal.set_balance(&sender_address, sender_balance);

        value.copy_from(&dest_addr);

        // TODO: add dest_addr as warm in the access list
        self.write_result(
            offset as u32,
            program_len,
            *remaining_gas,
            return_codes::SUCCESS_RETURN_CODE,
        );
        return_codes::SUCCESS_RETURN_CODE
    }

    pub extern "C" fn create(
        &mut self,
        size: u32,
        offset: u32,
        value: &mut U256,
        remaining_gas: &mut u64,
    ) -> u8 {
        self.create_aux(size, offset, value, remaining_gas, None)
    }

    pub extern "C" fn create2(
        &mut self,
        size: u32,
        offset: u32,
        value: &mut U256,
        remaining_gas: &mut u64,
        salt: &U256,
    ) -> u8 {
        self.create_aux(size, offset, value, remaining_gas, Some(salt))
    }

    pub extern "C" fn selfdestruct(&mut self, receiver_address: &U256) -> u64 {
        let sender_address = self.env.tx.get_address();
        let receiver_address = Address::from(receiver_address);

        let sender_balance = self
            .journal
            .get_account(&sender_address)
            .unwrap_or_default()
            .balance;

        let receiver_is_empty = match self.journal.get_account(&receiver_address) {
            Some(receiver) => {
                let is_empty = receiver.is_empty();
                self.journal
                    .set_balance(&receiver_address, receiver.balance + sender_balance);
                is_empty
            }
            None => {
                self.journal.new_account(receiver_address, sender_balance);
                true
            }
        };

        self.journal.set_balance(&sender_address, EU256::zero());

        if self.journal.get_account(&sender_address).is_some() {
            self.journal
                .set_status(&sender_address, AccountStatus::SelfDestructed);
        }

        if !sender_balance.is_zero() && receiver_is_empty {
            gas_cost::SELFDESTRUCT_DYNAMIC_GAS as u64
        } else {
            0
        }
        // TODO: add gas cost for cold addresses
    }

    pub extern "C" fn read_transient_storage(&mut self, stg_key: &U256, stg_value: &mut U256) {
        let key = stg_key.to_primitive_u256();
        let address = self.env.tx.get_address();

        let result = self
            .transient_storage
            .get(&(address, key))
            .cloned()
            .unwrap_or(EU256::zero());

        stg_value.hi = (result >> 128).low_u128();
        stg_value.lo = result.low_u128();
    }

    pub extern "C" fn write_transient_storage(&mut self, stg_key: &U256, stg_value: &mut U256) {
        let address = self.env.tx.get_address();

        let key = stg_key.to_primitive_u256();
        let value = stg_value.to_primitive_u256();
        self.transient_storage.insert((address, key), value);
    }
}

pub mod symbols {
    // Global variables
    pub const CONTEXT_IS_STATIC: &str = "evm_mlir__context_is_static";
    // Syscalls
    pub const WRITE_RESULT: &str = "evm_mlir__write_result";
    pub const EXTEND_MEMORY: &str = "evm_mlir__extend_memory";
    pub const KECCAK256_HASHER: &str = "evm_mlir__keccak256_hasher";
    pub const STORAGE_WRITE: &str = "evm_mlir__write_storage";
    pub const STORAGE_READ: &str = "evm_mlir__read_storage";
    pub const APPEND_LOG: &str = "evm_mlir__append_log";
    pub const APPEND_LOG_ONE_TOPIC: &str = "evm_mlir__append_log_with_one_topic";
    pub const APPEND_LOG_TWO_TOPICS: &str = "evm_mlir__append_log_with_two_topics";
    pub const APPEND_LOG_THREE_TOPICS: &str = "evm_mlir__append_log_with_three_topics";
    pub const APPEND_LOG_FOUR_TOPICS: &str = "evm_mlir__append_log_with_four_topics";
    pub const GET_CALLDATA_PTR: &str = "evm_mlir__get_calldata_ptr";
    pub const GET_CALLDATA_SIZE: &str = "evm_mlir__get_calldata_size";
    pub const GET_CODESIZE_FROM_ADDRESS: &str = "evm_mlir__get_codesize_from_address";
    pub const COPY_CODE_TO_MEMORY: &str = "evm_mlir__copy_code_to_memory";
    pub const GET_ADDRESS_PTR: &str = "evm_mlir__get_address_ptr";
    pub const GET_GASLIMIT: &str = "evm_mlir__get_gaslimit";
    pub const STORE_IN_CALLVALUE_PTR: &str = "evm_mlir__store_in_callvalue_ptr";
    pub const STORE_IN_BLOBBASEFEE_PTR: &str = "evm_mlir__store_in_blobbasefee_ptr";
    pub const GET_BLOB_HASH_AT_INDEX: &str = "evm_mlir__get_blob_hash_at_index";
    pub const STORE_IN_BALANCE: &str = "evm_mlir__store_in_balance";
    pub const GET_COINBASE_PTR: &str = "evm_mlir__get_coinbase_ptr";
    pub const STORE_IN_TIMESTAMP_PTR: &str = "evm_mlir__store_in_timestamp_ptr";
    pub const STORE_IN_BASEFEE_PTR: &str = "evm_mlir__store_in_basefee_ptr";
    pub const STORE_IN_CALLER_PTR: &str = "evm_mlir__store_in_caller_ptr";
    pub const GET_ORIGIN: &str = "evm_mlir__get_origin";
    pub const GET_CHAINID: &str = "evm_mlir__get_chainid";
    pub const STORE_IN_GASPRICE_PTR: &str = "evm_mlir__store_in_gasprice_ptr";
    pub const GET_BLOCK_NUMBER: &str = "evm_mlir__get_block_number";
    pub const STORE_IN_SELFBALANCE_PTR: &str = "evm_mlir__store_in_selfbalance_ptr";
    pub const COPY_EXT_CODE_TO_MEMORY: &str = "evm_mlir__copy_ext_code_to_memory";
    pub const GET_PREVRANDAO: &str = "evm_mlir__get_prevrandao";
    pub const GET_BLOCK_HASH: &str = "evm_mlir__get_block_hash";
    pub const GET_CODE_HASH: &str = "evm_mlir__get_code_hash";
    pub const CALL: &str = "evm_mlir__call";
    pub const CREATE: &str = "evm_mlir__create";
    pub const CREATE2: &str = "evm_mlir__create2";
    pub const GET_RETURN_DATA_SIZE: &str = "evm_mlir__get_return_data_size";
    pub const COPY_RETURN_DATA_INTO_MEMORY: &str = "evm_mlir__copy_return_data_into_memory";
    pub const TRANSIENT_STORAGE_READ: &str = "evm_mlir__transient_storage_read";
    pub const TRANSIENT_STORAGE_WRITE: &str = "evm_mlir__transient_storage_write";
    pub const SELFDESTRUCT: &str = "evm_mlir__selfdestruct";
}

impl<'c> SyscallContext<'c> {
    /// Registers all the syscalls as symbols in the execution engine
    ///
    /// This allows the generated code to call the syscalls by name.
    pub fn register_symbols(&self, engine: &ExecutionEngine) {
        unsafe {
            // Global variables
            engine.register_symbol(
                symbols::CONTEXT_IS_STATIC,
                &self.call_frame.ctx_is_static as *const bool as *mut (),
            );
            // Syscalls
            engine.register_symbol(
                symbols::WRITE_RESULT,
                SyscallContext::write_result as *const fn(*mut c_void, u32, u32, u64, u8)
                    as *mut (),
            );
            engine.register_symbol(
                symbols::KECCAK256_HASHER,
                SyscallContext::keccak256_hasher as *const fn(*mut c_void, u32, u32, *const U256)
                    as *mut (),
            );
            engine.register_symbol(
                symbols::EXTEND_MEMORY,
                SyscallContext::extend_memory as *const fn(*mut c_void, u32) as *mut (),
            );
            engine.register_symbol(
                symbols::STORAGE_READ,
                SyscallContext::read_storage as *const fn(*mut c_void, *const U256, *mut U256)
                    as *mut (),
            );
            engine.register_symbol(
                symbols::STORAGE_WRITE,
                SyscallContext::write_storage as *const fn(*mut c_void, *const U256, *const U256)
                    as *mut (),
            );
            engine.register_symbol(
                symbols::APPEND_LOG,
                SyscallContext::append_log as *const fn(*mut c_void, u32, u32) as *mut (),
            );
            engine.register_symbol(
                symbols::APPEND_LOG_ONE_TOPIC,
                SyscallContext::append_log_with_one_topic
                    as *const fn(*mut c_void, u32, u32, *const U256) as *mut (),
            );
            engine.register_symbol(
                symbols::APPEND_LOG_TWO_TOPICS,
                SyscallContext::append_log_with_two_topics
                    as *const fn(*mut c_void, u32, u32, *const U256, *const U256)
                    as *mut (),
            );
            engine.register_symbol(
                symbols::APPEND_LOG_THREE_TOPICS,
                SyscallContext::append_log_with_three_topics
                    as *const fn(*mut c_void, u32, u32, *const U256, *const U256, *const U256)
                    as *mut (),
            );
            engine.register_symbol(
                symbols::APPEND_LOG_FOUR_TOPICS,
                SyscallContext::append_log_with_four_topics
                    as *const fn(
                        *mut c_void,
                        u32,
                        u32,
                        *const U256,
                        *const U256,
                        *const U256,
                        *const U256,
                    ) as *mut (),
            );
            engine.register_symbol(
                symbols::CALL,
                SyscallContext::call
                    as *const fn(
                        *mut c_void,
                        u64,
                        *const U256,
                        *const U256,
                        u32,
                        u32,
                        u32,
                        u32,
                        u64,
                        *mut u64,
                        u8,
                    ) as *mut (),
            );
            engine.register_symbol(
                symbols::GET_CALLDATA_PTR,
                SyscallContext::get_calldata_ptr as *const fn(*mut c_void) as *mut (),
            );
            engine.register_symbol(
                symbols::GET_CALLDATA_SIZE,
                SyscallContext::get_calldata_size_syscall as *const fn(*mut c_void) as *mut (),
            );
            engine.register_symbol(
                symbols::EXTEND_MEMORY,
                SyscallContext::extend_memory as *const fn(*mut c_void, u32) as *mut (),
            );
            engine.register_symbol(
                symbols::COPY_CODE_TO_MEMORY,
                SyscallContext::copy_code_to_memory as *const fn(*mut c_void, u32, u32, u32)
                    as *mut (),
            );
            engine.register_symbol(
                symbols::GET_ORIGIN,
                SyscallContext::get_origin as *const fn(*mut c_void, *mut U256) as *mut (),
            );
            engine.register_symbol(
                symbols::GET_ADDRESS_PTR,
                SyscallContext::get_address_ptr as *const fn(*mut c_void) as *mut (),
            );
            engine.register_symbol(
                symbols::STORE_IN_CALLVALUE_PTR,
                SyscallContext::store_in_callvalue_ptr as *const fn(*mut c_void, *mut U256)
                    as *mut (),
            );
            engine.register_symbol(
                symbols::STORE_IN_BLOBBASEFEE_PTR,
                SyscallContext::store_in_blobbasefee_ptr
                    as *const extern "C" fn(&SyscallContext, *mut u128) -> ()
                    as *mut (),
            );
            engine.register_symbol(
                symbols::GET_CODESIZE_FROM_ADDRESS,
                SyscallContext::get_codesize_from_address
                    as *const fn(*mut c_void, *mut U256, *mut u64) as *mut (),
            );
            engine.register_symbol(
                symbols::GET_COINBASE_PTR,
                SyscallContext::get_coinbase_ptr as *const fn(*mut c_void) as *mut (),
            );
            engine.register_symbol(
                symbols::STORE_IN_TIMESTAMP_PTR,
                SyscallContext::store_in_timestamp_ptr as *const fn(*mut c_void, *mut U256)
                    as *mut (),
            );
            engine.register_symbol(
                symbols::STORE_IN_BASEFEE_PTR,
                SyscallContext::store_in_basefee_ptr as *const fn(*mut c_void, *mut U256)
                    as *mut (),
            );
            engine.register_symbol(
                symbols::STORE_IN_CALLER_PTR,
                SyscallContext::store_in_caller_ptr as *const fn(*mut c_void, *mut U256) as *mut (),
            );
            engine.register_symbol(
                symbols::GET_GASLIMIT,
                SyscallContext::get_gaslimit as *const fn(*mut c_void) as *mut (),
            );
            engine.register_symbol(
                symbols::STORE_IN_GASPRICE_PTR,
                SyscallContext::store_in_gasprice_ptr as *const fn(*mut c_void, *mut U256)
                    as *mut (),
            );
            engine.register_symbol(
                symbols::GET_BLOCK_NUMBER,
                SyscallContext::get_block_number as *const fn(*mut c_void, *mut U256) as *mut (),
            );
            engine.register_symbol(
                symbols::GET_PREVRANDAO,
                SyscallContext::get_prevrandao as *const fn(*mut c_void, *mut U256) as *mut (),
            );
            engine.register_symbol(
                symbols::GET_BLOB_HASH_AT_INDEX,
                SyscallContext::get_blob_hash_at_index
                    as *const fn(*mut c_void, *mut U256, *mut U256) as *mut (),
            );
            engine.register_symbol(
                symbols::GET_CHAINID,
                SyscallContext::get_chainid as *const extern "C" fn(&SyscallContext) -> u64
                    as *mut (),
            );
            engine.register_symbol(
                symbols::STORE_IN_BALANCE,
                SyscallContext::store_in_balance as *const fn(*mut c_void, *const U256, *mut U256)
                    as *mut (),
            );
            engine.register_symbol(
                symbols::STORE_IN_SELFBALANCE_PTR,
                SyscallContext::store_in_selfbalance_ptr
                    as *const extern "C" fn(&SyscallContext) -> u64 as *mut (),
            );
            engine.register_symbol(
                symbols::COPY_EXT_CODE_TO_MEMORY,
                SyscallContext::copy_ext_code_to_memory
                    as *const extern "C" fn(*mut c_void, *mut U256, u32, u32, u32) -> u64
                    as *mut (),
            );
            engine.register_symbol(
                symbols::GET_BLOCK_HASH,
                SyscallContext::get_block_hash as *const fn(*mut c_void, *mut U256) as *mut (),
            );

            engine.register_symbol(
                symbols::GET_CODE_HASH,
                SyscallContext::get_code_hash as *const fn(*mut c_void, *mut U256) -> u64
                    as *mut (),
            );

            engine.register_symbol(
                symbols::CREATE,
                SyscallContext::create
                    as *const extern "C" fn(*mut c_void, u32, u32, *mut U256, *mut u64)
                    as *mut (),
            );

            engine.register_symbol(
                symbols::CREATE2,
                SyscallContext::create2
                    as *const extern "C" fn(*mut c_void, u32, u32, *mut U256, *mut u64, *mut U256)
                    as *mut (),
            );

            engine.register_symbol(
                symbols::GET_RETURN_DATA_SIZE,
                SyscallContext::get_return_data_size as *const fn(*mut c_void) as *mut (),
            );
            engine.register_symbol(
                symbols::COPY_RETURN_DATA_INTO_MEMORY,
                SyscallContext::copy_return_data_into_memory
                    as *const fn(*mut c_void, u32, u32, u32) as *mut (),
            );

            engine.register_symbol(
                symbols::SELFDESTRUCT,
                SyscallContext::selfdestruct as *const fn(*mut c_void, *mut U256) as *mut (),
            );

            engine.register_symbol(
                symbols::TRANSIENT_STORAGE_READ,
                SyscallContext::read_transient_storage
                    as *const fn(*const c_void, *const U256, *mut U256) as *mut (),
            );

            engine.register_symbol(
                symbols::TRANSIENT_STORAGE_WRITE,
                SyscallContext::write_transient_storage
                    as *const fn(*const c_void, *const U256, *mut U256) as *mut (),
            );
        }
    }
}

/// MLIR util for declaring syscalls
pub(crate) mod mlir {
    use melior::{
        dialect::{
            func,
            llvm::{attributes::Linkage, r#type::pointer},
        },
        ir::{
            attribute::{FlatSymbolRefAttribute, StringAttribute, TypeAttribute},
            r#type::{FunctionType, IntegerType},
            Block, Identifier, Location, Module as MeliorModule, Region, Value,
        },
        Context as MeliorContext,
    };

    use crate::{errors::CodegenError, utils::llvm_mlir};

    use super::symbols;

    pub(crate) fn declare_symbols(context: &MeliorContext, module: &MeliorModule) {
        let location = Location::unknown(context);

        // Type declarations
        let ptr_type = pointer(context, 0);
        let uint8 = IntegerType::new(context, 8).into();
        let uint32 = IntegerType::new(context, 32).into();
        let uint64 = IntegerType::new(context, 64).into();

        let attributes = &[(
            Identifier::new(context, "sym_visibility"),
            StringAttribute::new(context, "private").into(),
        )];

        // Globals declaration
        module.body().append_operation(llvm_mlir::global(
            context,
            symbols::CONTEXT_IS_STATIC,
            ptr_type,
            Linkage::External,
            location,
        ));
        // Syscall declarations
        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::WRITE_RESULT),
            TypeAttribute::new(
                FunctionType::new(context, &[ptr_type, uint32, uint32, uint64, uint8], &[]).into(),
            ),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::KECCAK256_HASHER),
            TypeAttribute::new(
                FunctionType::new(context, &[ptr_type, uint32, uint32, ptr_type], &[]).into(),
            ),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::GET_CALLDATA_PTR),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type], &[ptr_type]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::GET_CALLDATA_SIZE),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type], &[uint32]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::GET_CHAINID),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type], &[uint64]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::STORE_IN_CALLVALUE_PTR),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type, ptr_type], &[]).into()),
            Region::new(),
            attributes,
            location,
        ));
        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::STORE_IN_CALLER_PTR),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type, ptr_type], &[]).into()),
            Region::new(),
            attributes,
            location,
        ));
        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::STORE_IN_GASPRICE_PTR),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type, ptr_type], &[]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::STORE_IN_SELFBALANCE_PTR),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type, ptr_type], &[]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::STORE_IN_BLOBBASEFEE_PTR),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type, ptr_type], &[]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::GET_GASLIMIT),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type], &[uint64]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::EXTEND_MEMORY),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type, uint32], &[ptr_type]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::COPY_CODE_TO_MEMORY),
            TypeAttribute::new(
                FunctionType::new(context, &[ptr_type, uint32, uint32, uint32], &[]).into(),
            ),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::STORAGE_READ),
            r#TypeAttribute::new(
                FunctionType::new(context, &[ptr_type, ptr_type, ptr_type], &[uint64]).into(),
            ),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::STORAGE_WRITE),
            r#TypeAttribute::new(
                FunctionType::new(context, &[ptr_type, ptr_type, ptr_type], &[uint64]).into(),
            ),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::APPEND_LOG),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type, uint32, uint32], &[]).into()),
            Region::new(),
            attributes,
            location,
        ));
        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::APPEND_LOG_ONE_TOPIC),
            TypeAttribute::new(
                FunctionType::new(context, &[ptr_type, uint32, uint32, ptr_type], &[]).into(),
            ),
            Region::new(),
            attributes,
            location,
        ));
        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::APPEND_LOG_TWO_TOPICS),
            TypeAttribute::new(
                FunctionType::new(
                    context,
                    &[ptr_type, uint32, uint32, ptr_type, ptr_type],
                    &[],
                )
                .into(),
            ),
            Region::new(),
            attributes,
            location,
        ));
        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::APPEND_LOG_THREE_TOPICS),
            TypeAttribute::new(
                FunctionType::new(
                    context,
                    &[ptr_type, uint32, uint32, ptr_type, ptr_type, ptr_type],
                    &[],
                )
                .into(),
            ),
            Region::new(),
            attributes,
            location,
        ));
        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::APPEND_LOG_FOUR_TOPICS),
            TypeAttribute::new(
                FunctionType::new(
                    context,
                    &[
                        ptr_type, uint32, uint32, ptr_type, ptr_type, ptr_type, ptr_type,
                    ],
                    &[],
                )
                .into(),
            ),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::GET_ORIGIN),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type, ptr_type], &[]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::GET_COINBASE_PTR),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type], &[ptr_type]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::GET_BLOCK_NUMBER),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type, ptr_type], &[]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::GET_CODESIZE_FROM_ADDRESS),
            TypeAttribute::new(
                FunctionType::new(context, &[ptr_type, ptr_type, ptr_type], &[uint64]).into(),
            ),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::GET_ADDRESS_PTR),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type], &[ptr_type]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::GET_PREVRANDAO),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type, ptr_type], &[]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::STORE_IN_TIMESTAMP_PTR),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type, ptr_type], &[]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::STORE_IN_BASEFEE_PTR),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type, ptr_type], &[]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::CALL),
            TypeAttribute::new(
                FunctionType::new(
                    context,
                    &[
                        ptr_type, uint64, ptr_type, ptr_type, uint32, uint32, uint32, uint32,
                        uint64, ptr_type, uint8,
                    ],
                    &[uint8],
                )
                .into(),
            ),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::STORE_IN_BALANCE),
            TypeAttribute::new(
                FunctionType::new(context, &[ptr_type, ptr_type, ptr_type], &[uint64]).into(),
            ),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::COPY_EXT_CODE_TO_MEMORY),
            TypeAttribute::new(
                FunctionType::new(
                    context,
                    &[ptr_type, ptr_type, uint32, uint32, uint32],
                    &[uint64],
                )
                .into(),
            ),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::GET_BLOB_HASH_AT_INDEX),
            TypeAttribute::new(
                FunctionType::new(context, &[ptr_type, ptr_type, ptr_type], &[]).into(),
            ),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::GET_BLOCK_HASH),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type, ptr_type], &[]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::GET_CODE_HASH),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type, ptr_type], &[uint64]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::CREATE),
            TypeAttribute::new(
                FunctionType::new(
                    context,
                    &[ptr_type, uint32, uint32, ptr_type, ptr_type],
                    &[uint8],
                )
                .into(),
            ),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::CREATE2),
            TypeAttribute::new(
                FunctionType::new(
                    context,
                    &[ptr_type, uint32, uint32, ptr_type, ptr_type, ptr_type],
                    &[uint8],
                )
                .into(),
            ),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::GET_RETURN_DATA_SIZE),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type], &[uint32]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::COPY_RETURN_DATA_INTO_MEMORY),
            TypeAttribute::new(
                FunctionType::new(context, &[ptr_type, uint32, uint32, uint32], &[]).into(),
            ),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::SELFDESTRUCT),
            TypeAttribute::new(FunctionType::new(context, &[ptr_type, ptr_type], &[uint64]).into()),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::TRANSIENT_STORAGE_READ),
            r#TypeAttribute::new(
                FunctionType::new(context, &[ptr_type, ptr_type, ptr_type], &[]).into(),
            ),
            Region::new(),
            attributes,
            location,
        ));

        module.body().append_operation(func::func(
            context,
            StringAttribute::new(context, symbols::TRANSIENT_STORAGE_WRITE),
            r#TypeAttribute::new(
                FunctionType::new(context, &[ptr_type, ptr_type, ptr_type], &[]).into(),
            ),
            Region::new(),
            attributes,
            location,
        ));
    }

    /// Stores the return values in the syscall context
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn write_result_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &Block,
        offset: Value,
        size: Value,
        gas: Value,
        reason: Value,
        location: Location,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::WRITE_RESULT),
            &[syscall_ctx, offset, size, gas, reason],
            &[],
            location,
        ));
    }

    pub(crate) fn keccak256_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        offset: Value<'c, 'c>,
        size: Value<'c, 'c>,
        hash_ptr: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::KECCAK256_HASHER),
            &[syscall_ctx, offset, size, hash_ptr],
            &[],
            location,
        ));
    }

    pub(crate) fn get_calldata_size_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let uint32 = IntegerType::new(mlir_ctx, 32).into();
        let value = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::GET_CALLDATA_SIZE),
                &[syscall_ctx],
                &[uint32],
                location,
            ))
            .result(0)?;
        Ok(value.into())
    }

    /// Returns a pointer to the start of the calldata
    pub(crate) fn get_calldata_ptr_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let ptr_type = pointer(mlir_ctx, 0);
        let value = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::GET_CALLDATA_PTR),
                &[syscall_ctx],
                &[ptr_type],
                location,
            ))
            .result(0)?;
        Ok(value.into())
    }

    pub(crate) fn get_gaslimit<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let uint64 = IntegerType::new(mlir_ctx, 64).into();
        let value = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::GET_GASLIMIT),
                &[syscall_ctx],
                &[uint64],
                location,
            ))
            .result(0)?;
        Ok(value.into())
    }

    pub(crate) fn get_chainid_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let uint64 = IntegerType::new(mlir_ctx, 64).into();
        let value = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::GET_CHAINID),
                &[syscall_ctx],
                &[uint64],
                location,
            ))
            .result(0)?;
        Ok(value.into())
    }

    pub(crate) fn store_in_callvalue_ptr<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        location: Location<'c>,
        callvalue_ptr: Value<'c, 'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::STORE_IN_CALLVALUE_PTR),
            &[syscall_ctx, callvalue_ptr],
            &[],
            location,
        ));
    }

    pub(crate) fn store_in_blobbasefee_ptr<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        location: Location<'c>,
        blob_base_fee_ptr: Value<'c, 'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::STORE_IN_BLOBBASEFEE_PTR),
            &[syscall_ctx, blob_base_fee_ptr],
            &[],
            location,
        ));
    }

    pub(crate) fn store_in_gasprice_ptr<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        location: Location<'c>,
        gasprice_ptr: Value<'c, 'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::STORE_IN_GASPRICE_PTR),
            &[syscall_ctx, gasprice_ptr],
            &[],
            location,
        ));
    }

    pub(crate) fn store_in_caller_ptr<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        location: Location<'c>,
        caller_ptr: Value<'c, 'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::STORE_IN_CALLER_PTR),
            &[syscall_ctx, caller_ptr],
            &[],
            location,
        ));
    }

    /// Extends the memory segment of the syscall context.
    /// Returns a pointer to the start of the memory segment.
    pub(crate) fn extend_memory_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        new_size: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let ptr_type = pointer(mlir_ctx, 0);
        let value = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::EXTEND_MEMORY),
                &[syscall_ctx, new_size],
                &[ptr_type],
                location,
            ))
            .result(0)?;
        Ok(value.into())
    }

    pub(crate) fn store_in_selfbalance_ptr<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        location: Location<'c>,
        balance_ptr: Value<'c, 'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::STORE_IN_SELFBALANCE_PTR),
            &[syscall_ctx, balance_ptr],
            &[],
            location,
        ));
    }

    /// Reads the storage given a key
    pub(crate) fn storage_read_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        key: Value<'c, 'c>,
        value: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let uint64 = IntegerType::new(mlir_ctx, 64);
        let result = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::STORAGE_READ),
                &[syscall_ctx, key, value],
                &[uint64.into()],
                location,
            ))
            .result(0)?;
        Ok(result.into())
    }

    /// Writes the storage given a key value pair
    pub(crate) fn storage_write_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        key: Value<'c, 'c>,
        value: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let uint64 = IntegerType::new(mlir_ctx, 64);
        let value = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::STORAGE_WRITE),
                &[syscall_ctx, key, value],
                &[uint64.into()],
                location,
            ))
            .result(0)?;
        Ok(value.into())
    }

    pub(crate) fn transient_storage_read_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        key: Value<'c, 'c>,
        value: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::TRANSIENT_STORAGE_READ),
            &[syscall_ctx, key, value],
            &[],
            location,
        ));
    }

    pub(crate) fn transient_storage_write_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        key: Value<'c, 'c>,
        value: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::TRANSIENT_STORAGE_WRITE),
            &[syscall_ctx, key, value],
            &[],
            location,
        ));
    }

    /// Receives log data and appends a log to the logs vector
    pub(crate) fn append_log_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        data: Value<'c, 'c>,
        size: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::APPEND_LOG),
            &[syscall_ctx, data, size],
            &[],
            location,
        ));
    }

    /// Receives log data and a topic and appends a log to the logs vector
    pub(crate) fn append_log_with_one_topic_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        data: Value<'c, 'c>,
        size: Value<'c, 'c>,
        topic: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::APPEND_LOG_ONE_TOPIC),
            &[syscall_ctx, data, size, topic],
            &[],
            location,
        ));
    }

    /// Receives log data, two topics and appends a log to the logs vector
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_log_with_two_topics_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        data: Value<'c, 'c>,
        size: Value<'c, 'c>,
        topic1_ptr: Value<'c, 'c>,
        topic2_ptr: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::APPEND_LOG_TWO_TOPICS),
            &[syscall_ctx, data, size, topic1_ptr, topic2_ptr],
            &[],
            location,
        ));
    }

    /// Receives log data, three topics and appends a log to the logs vector
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_log_with_three_topics_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        data: Value<'c, 'c>,
        size: Value<'c, 'c>,
        topic1_ptr: Value<'c, 'c>,
        topic2_ptr: Value<'c, 'c>,
        topic3_ptr: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::APPEND_LOG_THREE_TOPICS),
            &[syscall_ctx, data, size, topic1_ptr, topic2_ptr, topic3_ptr],
            &[],
            location,
        ));
    }

    /// Receives log data, three topics and appends a log to the logs vector
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn append_log_with_four_topics_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        data: Value<'c, 'c>,
        size: Value<'c, 'c>,
        topic1_ptr: Value<'c, 'c>,
        topic2_ptr: Value<'c, 'c>,
        topic3_ptr: Value<'c, 'c>,
        topic4_ptr: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::APPEND_LOG_FOUR_TOPICS),
            &[
                syscall_ctx,
                data,
                size,
                topic1_ptr,
                topic2_ptr,
                topic3_ptr,
                topic4_ptr,
            ],
            &[],
            location,
        ));
    }

    pub(crate) fn get_origin_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        address_pointer: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::GET_ORIGIN),
            &[syscall_ctx, address_pointer],
            &[],
            location,
        ));
    }

    /// Returns a pointer to the coinbase address.
    pub(crate) fn get_coinbase_ptr_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let ptr_type = pointer(mlir_ctx, 0);
        let value = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::GET_COINBASE_PTR),
                &[syscall_ctx],
                &[ptr_type],
                location,
            ))
            .result(0)?;
        Ok(value.into())
    }

    /// Returns the block number.
    #[allow(unused)]
    pub(crate) fn get_block_number_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        number: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::GET_BLOCK_NUMBER),
            &[syscall_ctx, number],
            &[],
            location,
        ));
    }

    pub(crate) fn copy_code_to_memory_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        offset: Value,
        size: Value,
        dest_offset: Value,
        location: Location<'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::COPY_CODE_TO_MEMORY),
            &[syscall_ctx, offset, size, dest_offset],
            &[],
            location,
        ));
    }

    /// Returns a pointer to the address of the current executing contract
    #[allow(unused)]
    pub(crate) fn get_address_ptr_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let uint256 = IntegerType::new(mlir_ctx, 256);
        let ptr_type = pointer(mlir_ctx, 0);
        let value = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::GET_ADDRESS_PTR),
                &[syscall_ctx],
                &[ptr_type],
                location,
            ))
            .result(0)?;
        Ok(value.into())
    }

    /// Stores the current block's timestamp in the `timestamp_ptr`.
    pub(crate) fn store_in_timestamp_ptr<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        location: Location<'c>,
        timestamp_ptr: Value<'c, 'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::STORE_IN_TIMESTAMP_PTR),
            &[syscall_ctx, timestamp_ptr],
            &[],
            location,
        ));
    }

    #[allow(unused)]
    pub(crate) fn store_in_basefee_ptr_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        basefee_ptr: Value<'c, 'c>,
        block: &'c Block,
        location: Location<'c>,
    ) {
        block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::STORE_IN_BASEFEE_PTR),
                &[syscall_ctx, basefee_ptr],
                &[],
                location,
            ))
            .result(0);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn call_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        location: Location<'c>,
        gas: Value<'c, 'c>,
        address: Value<'c, 'c>,
        value_ptr: Value<'c, 'c>,
        args_offset: Value<'c, 'c>,
        args_size: Value<'c, 'c>,
        ret_offset: Value<'c, 'c>,
        ret_size: Value<'c, 'c>,
        available_gas: Value<'c, 'c>,
        remaining_gas_ptr: Value<'c, 'c>,
        call_type: Value<'c, 'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let uint8 = IntegerType::new(mlir_ctx, 8).into();
        let result = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::CALL),
                &[
                    syscall_ctx,
                    gas,
                    address,
                    value_ptr,
                    args_offset,
                    args_size,
                    ret_offset,
                    ret_size,
                    available_gas,
                    remaining_gas_ptr,
                    call_type,
                ],
                &[uint8],
                location,
            ))
            .result(0)?;

        Ok(result.into())
    }

    pub(crate) fn store_in_balance_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        address: Value<'c, 'c>,
        balance: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let uint64 = IntegerType::new(mlir_ctx, 64).into();

        let value = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::STORE_IN_BALANCE),
                &[syscall_ctx, address, balance],
                &[uint64],
                location,
            ))
            .result(0)?;
        Ok(value.into())
    }

    /// Receives an account address and copies the corresponding bytecode
    /// to memory.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn copy_ext_code_to_memory_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        address_ptr: Value<'c, 'c>,
        offset: Value<'c, 'c>,
        size: Value<'c, 'c>,
        dest_offset: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let uint64 = IntegerType::new(mlir_ctx, 64).into();

        let value = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::COPY_EXT_CODE_TO_MEMORY),
                &[syscall_ctx, address_ptr, offset, size, dest_offset],
                &[uint64],
                location,
            ))
            .result(0)?;
        Ok(value.into())
    }

    pub(crate) fn get_prevrandao_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        prevrandao_ptr: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::GET_PREVRANDAO),
            &[syscall_ctx, prevrandao_ptr],
            &[],
            location,
        ));
    }

    pub(crate) fn get_codesize_from_address_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        address: Value<'c, 'c>,
        gas: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let uint64 = IntegerType::new(mlir_ctx, 64).into();
        let value = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::GET_CODESIZE_FROM_ADDRESS),
                &[syscall_ctx, address, gas],
                &[uint64],
                location,
            ))
            .result(0)?;
        Ok(value.into())
    }

    pub(crate) fn get_blob_hash_at_index_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        index: Value<'c, 'c>,
        blobhash: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::GET_BLOB_HASH_AT_INDEX),
            &[syscall_ctx, index, blobhash],
            &[],
            location,
        ));
    }

    pub(crate) fn get_block_hash_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        block_number: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::GET_BLOCK_HASH),
            &[syscall_ctx, block_number],
            &[],
            location,
        ));
    }

    pub(crate) fn get_code_hash_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        address: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let uint64 = IntegerType::new(mlir_ctx, 64).into();

        let value = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::GET_CODE_HASH),
                &[syscall_ctx, address],
                &[uint64],
                location,
            ))
            .result(0)?;
        Ok(value.into())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        size: Value<'c, 'c>,
        offset: Value<'c, 'c>,
        value: Value<'c, 'c>,
        remaining_gas: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let uint8 = IntegerType::new(mlir_ctx, 8).into();
        let result = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::CREATE),
                &[syscall_ctx, size, offset, value, remaining_gas],
                &[uint8],
                location,
            ))
            .result(0)?;
        Ok(result.into())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create2_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        size: Value<'c, 'c>,
        offset: Value<'c, 'c>,
        value: Value<'c, 'c>,
        remaining_gas: Value<'c, 'c>,
        salt: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let uint8 = IntegerType::new(mlir_ctx, 8).into();
        let result = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::CREATE2),
                &[syscall_ctx, size, offset, value, remaining_gas, salt],
                &[uint8],
                location,
            ))
            .result(0)?;
        Ok(result.into())
    }

    pub(crate) fn get_return_data_size<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let uint32 = IntegerType::new(mlir_ctx, 32).into();
        let result = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::GET_RETURN_DATA_SIZE),
                &[syscall_ctx],
                &[uint32],
                location,
            ))
            .result(0)?;

        Ok(result.into())
    }

    pub(crate) fn copy_return_data_into_memory<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        dest_offset: Value<'c, 'c>,
        offset: Value<'c, 'c>,
        size: Value<'c, 'c>,
        location: Location<'c>,
    ) {
        block.append_operation(func::call(
            mlir_ctx,
            FlatSymbolRefAttribute::new(mlir_ctx, symbols::COPY_RETURN_DATA_INTO_MEMORY),
            &[syscall_ctx, dest_offset, offset, size],
            &[],
            location,
        ));
    }

    pub(crate) fn selfdestruct_syscall<'c>(
        mlir_ctx: &'c MeliorContext,
        syscall_ctx: Value<'c, 'c>,
        block: &'c Block,
        address: Value<'c, 'c>,
        location: Location<'c>,
    ) -> Result<Value<'c, 'c>, CodegenError> {
        let uint64 = IntegerType::new(mlir_ctx, 64).into();

        let result = block
            .append_operation(func::call(
                mlir_ctx,
                FlatSymbolRefAttribute::new(mlir_ctx, symbols::SELFDESTRUCT),
                &[syscall_ctx, address],
                &[uint64],
                location,
            ))
            .result(0)?;

        Ok(result.into())
    }
}
