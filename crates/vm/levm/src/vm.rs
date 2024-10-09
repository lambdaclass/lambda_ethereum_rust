use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use ethers::utils::keccak256;

use crate::{
    block::BlockEnv,
    call_frame::CallFrame,
    constants::*,
    opcodes::Opcode,
    primitives::{Address, Bytes, H256, U256},
    transaction::{TransactTo, TxEnv},
    vm_result::{
        AccountInfo, AccountStatus, ExecutionResult, ExitStatusCode, OpcodeSuccess, Output,
        ResultAndState, ResultReason, StateAccount, SuccessReason, VMError,
    },
};
extern crate ethereum_rust_rlp;
use ethereum_rust_rlp::encode::RLPEncode;
use ethereum_types::H160;
use sha3::{Digest, Keccak256};

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Account {
    pub address: Address,
    pub balance: U256,
    pub bytecode: Bytes,
    pub storage: HashMap<U256, StorageSlot>,
    pub nonce: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StorageSlot {
    pub original_value: U256,
    pub current_value: U256,
    pub is_cold: bool,
}

impl Account {
    pub fn new(
        address: Address,
        balance: U256,
        bytecode: Bytes,
        nonce: u64,
        storage: HashMap<U256, StorageSlot>,
    ) -> Self {
        Self {
            address,
            balance,
            bytecode,
            storage,
            nonce,
        }
    }

    pub fn has_code(&self) -> bool {
        !(self.bytecode.is_empty()
            || self.bytecode_hash() == H256::from_str(EMPTY_CODE_HASH_STR).unwrap())
    }

    pub fn bytecode_hash(&self) -> H256 {
        let hash = keccak256(self.bytecode.as_ref());
        H256::from(hash)
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

    pub fn with_nonce(mut self, nonce: u64) -> Self {
        self.nonce = nonce;
        self
    }

    pub fn increment_nonce(&mut self) {
        self.nonce += 1;
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

    pub fn increment_account_nonce(&mut self, address: &Address) {
        if let Some(acc) = self.accounts.get_mut(address) {
            acc.increment_nonce()
        }
    }

    pub fn into_state(&self) -> HashMap<Address, StateAccount> {
        self.accounts
            .iter()
            .map(|(address, acc)| {
                let code_hash = if acc.has_code() {
                    acc.bytecode_hash()
                } else {
                    H256::from_str(EMPTY_CODE_HASH_STR).unwrap()
                };

                let storage: HashMap<U256, StorageSlot> = acc
                    .storage
                    .iter()
                    .map(|(&key, slot)| {
                        (
                            key,
                            StorageSlot {
                                original_value: slot.original_value,
                                current_value: slot.current_value,
                                is_cold: false,
                            },
                        )
                    })
                    .collect();
                (
                    *address,
                    StateAccount {
                        info: AccountInfo {
                            balance: acc.balance,
                            nonce: acc.nonce,
                            code_hash,
                            code: acc.bytecode.clone(),
                        },
                        storage,
                        status: AccountStatus::Loaded,
                    },
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
// TODO: https://github.com/lambdaclass/ethereum_rust/issues/604
pub struct Substate {
    pub warm_addresses: HashSet<Address>,
}

#[derive(Debug, Default, Clone)]
pub struct Environment {
    /// The sender address of the transaction that originated
    /// this execution.
    // origin: Address,
    /// The price of gas paid by the signer of the transaction
    /// that originated this execution.
    // gas_price: u64,
    pub gas_limit: u64,
    pub consumed_gas: u64,
    refunded_gas: u64,
    /// The block header of the present block.
    pub block: BlockEnv,
}

#[derive(Debug, Clone, Default)]
pub struct VM {
    pub call_frames: Vec<CallFrame>,
    pub env: Environment,
    /// Information that is acted upon immediately following the
    /// transaction.
    pub accrued_substate: Substate,
    /// Mapping between addresses (160-bit identifiers) and account
    /// states.
    pub db: Db,
}

fn address_to_word(address: Address) -> U256 {
    // This unwrap can't panic, as Address are 20 bytes long and U256 use 32 bytes
    U256::from_str(&format!("{address:?}")).unwrap()
}

impl VM {
    pub fn new(tx_env: TxEnv, block_env: BlockEnv, db: Db) -> Self {
        let bytecode = match tx_env.transact_to {
            TransactTo::Call(addr) => db.get_account_bytecode(&addr),
            TransactTo::Create => {
                todo!()
            }
        };

        let to = match tx_env.transact_to {
            TransactTo::Call(addr) => addr,
            TransactTo::Create => tx_env.msg_sender,
        };

        let code_addr = match tx_env.transact_to {
            TransactTo::Call(addr) => addr,
            TransactTo::Create => todo!(),
        };

        // TODO: this is mostly placeholder
        let initial_call_frame = CallFrame::new(
            tx_env.msg_sender,
            to,
            code_addr,
            None,
            bytecode,
            tx_env.value,
            tx_env.data,
            false,
            U256::zero(),
            0,
        );

        let env = Environment {
            block: block_env,
            consumed_gas: TX_BASE_COST,
            gas_limit: u64::MAX,
            refunded_gas: 0,
        };

        Self {
            call_frames: vec![initial_call_frame],
            db,
            env,
            accrued_substate: Substate::default(),
        }
    }

    pub fn write_success_result(
        current_call_frame: CallFrame,
        reason: ResultReason,
        gas_used: u64,
        gas_refunded: u64,
    ) -> ExecutionResult {
        match reason {
            ResultReason::Stop => ExecutionResult::Success {
                reason: SuccessReason::Stop,
                logs: current_call_frame.logs.clone(),
                return_data: current_call_frame.returndata.clone(),
                gas_used,
                output: Output::Call(current_call_frame.returndata.clone()),
                gas_refunded,
            },
            ResultReason::Return => ExecutionResult::Success {
                reason: SuccessReason::Return,
                logs: current_call_frame.logs.clone(),
                return_data: current_call_frame.returndata.clone(),
                gas_used,
                output: Output::Call(current_call_frame.returndata.clone()),
                gas_refunded,
            },
            ResultReason::Revert => ExecutionResult::Revert {
                reason: VMError::FatalError,
                gas_used,
                output: current_call_frame.returndata.clone(),
            },
        }
    }

    pub fn get_result(&mut self, res: ExecutionResult) -> Result<ResultAndState, VMError> {
        let gas_used = self.env.consumed_gas;

        // TODO: Probably here we need to add the access_list_cost to gas_used, but we need a refactor of most tests
        let gas_refunded = self.env.refunded_gas.min(gas_used / GAS_REFUND_DENOMINATOR);

        let exis_status_code = match res {
            ExecutionResult::Success { reason, .. } => match reason {
                SuccessReason::Stop => ExitStatusCode::Stop,
                SuccessReason::Return => ExitStatusCode::Return,
                SuccessReason::SelfDestruct => todo!(),
                SuccessReason::EofReturnContract => todo!(),
            },
            ExecutionResult::Revert { .. } => ExitStatusCode::Revert,
            ExecutionResult::Halt { .. } => ExitStatusCode::Error,
        };

        let current_call_frame = self.current_call_frame_mut();

        let return_values = current_call_frame.returndata.clone();

        let result = match exis_status_code {
            ExitStatusCode::Return => ExecutionResult::Success {
                reason: SuccessReason::Return,
                gas_used,
                gas_refunded,
                output: Output::Call(return_values.clone()), // TODO: add case Output::Create
                logs: res.logs().to_vec(),
                return_data: return_values.clone(),
            },
            ExitStatusCode::Stop => ExecutionResult::Success {
                reason: SuccessReason::Stop,
                gas_used,
                gas_refunded,
                output: Output::Call(return_values.clone()), // TODO: add case Output::Create
                logs: res.logs().to_vec(),
                return_data: return_values.clone(),
            },
            ExitStatusCode::Revert => ExecutionResult::Revert {
                output: return_values,
                gas_used,
                reason: res.reason(),
            },
            ExitStatusCode::Error | ExitStatusCode::Default => ExecutionResult::Halt {
                reason: res.reason(),
                gas_used,
            },
        };

        // TODO: Check if this is ok
        let state = self.db.into_state();

        Ok(ResultAndState { result, state })
    }

    pub fn execute(&mut self, _initial_gas_consumed: u64) -> Result<ExecutionResult, VMError> {
        let mut current_call_frame = self.call_frames.pop().ok_or(VMError::FatalError)?; // if this happens during execution, we are cooked 💀
        loop {
            let opcode = current_call_frame.next_opcode().unwrap_or(Opcode::STOP);
            let op_result: Result<OpcodeSuccess, VMError> = match opcode {
                Opcode::STOP => self.op_stop(&mut current_call_frame),
                Opcode::ADD => self.op_add(&mut current_call_frame),
                Opcode::MUL => self.op_mul(&mut current_call_frame),
                Opcode::SUB => self.op_sub(&mut current_call_frame),
                Opcode::DIV => self.op_div(&mut current_call_frame),
                Opcode::SDIV => self.op_sdiv(&mut current_call_frame),
                Opcode::MOD => self.op_mod(&mut current_call_frame),
                Opcode::SMOD => self.op_smod(&mut current_call_frame),
                Opcode::ADDMOD => self.op_addmod(&mut current_call_frame),
                Opcode::MULMOD => self.op_mulmod(&mut current_call_frame),
                Opcode::EXP => self.op_exp(&mut current_call_frame),
                Opcode::SIGNEXTEND => self.op_signextend(&mut current_call_frame),
                Opcode::LT => self.op_lt(&mut current_call_frame),
                Opcode::GT => self.op_gt(&mut current_call_frame),
                Opcode::SLT => self.op_slt(&mut current_call_frame),
                Opcode::SGT => self.op_sgt(&mut current_call_frame),
                Opcode::EQ => self.op_eq(&mut current_call_frame),
                Opcode::ISZERO => self.op_iszero(&mut current_call_frame),
                Opcode::KECCAK256 => self.op_keccak256(&mut current_call_frame),
                Opcode::CALLDATALOAD => self.op_calldataload(&mut current_call_frame),
                Opcode::CALLDATASIZE => self.op_calldatasize(&mut current_call_frame),
                Opcode::CALLDATACOPY => self.op_calldatacopy(&mut current_call_frame),
                Opcode::RETURNDATASIZE => self.op_returndatasize(&mut current_call_frame),
                Opcode::RETURNDATACOPY => self.op_returndatacopy(&mut current_call_frame),
                Opcode::JUMP => self.op_jump(&mut current_call_frame),
                Opcode::JUMPI => self.op_jumpi(&mut current_call_frame),
                Opcode::JUMPDEST => self.op_jumpdest(),
                Opcode::PC => self.op_pc(&mut current_call_frame),
                Opcode::BLOCKHASH => self.op_blockhash(&mut current_call_frame),
                Opcode::COINBASE => self.op_coinbase(&mut current_call_frame),
                Opcode::TIMESTAMP => self.op_timestamp(&mut current_call_frame),
                Opcode::NUMBER => self.op_number(&mut current_call_frame),
                Opcode::PREVRANDAO => self.op_prevrandao(&mut current_call_frame),
                Opcode::GASLIMIT => self.op_gaslimit(&mut current_call_frame),
                Opcode::CHAINID => self.op_chainid(&mut current_call_frame),
                Opcode::SELFBALANCE => self.op_selfbalance(&mut current_call_frame),
                Opcode::BASEFEE => self.op_basefee(&mut current_call_frame),
                Opcode::BLOBHASH => self.op_blobhash(&mut current_call_frame),
                Opcode::BLOBBASEFEE => self.op_blobbasefee(&mut current_call_frame),
                Opcode::PUSH0 => self.op_push0(&mut current_call_frame),
                // PUSHn
                op if (Opcode::PUSH1..=Opcode::PUSH32).contains(&op) => {
                    self.op_push(&mut current_call_frame, op)
                }
                Opcode::AND => self.op_and(&mut current_call_frame),
                Opcode::OR => self.op_or(&mut current_call_frame),
                Opcode::XOR => self.op_xor(&mut current_call_frame),
                Opcode::NOT => self.op_not(&mut current_call_frame),
                Opcode::BYTE => self.op_byte(&mut current_call_frame),
                Opcode::SHL => self.op_shl(&mut current_call_frame),
                Opcode::SHR => self.op_shr(&mut current_call_frame),
                Opcode::SAR => self.op_sar(&mut current_call_frame),
                // DUPn
                op if (Opcode::DUP1..=Opcode::DUP16).contains(&op) => {
                    self.op_dup(&mut current_call_frame, op)
                }
                // SWAPn
                op if (Opcode::SWAP1..=Opcode::SWAP16).contains(&op) => {
                    self.op_swap(&mut current_call_frame, op)
                }
                Opcode::POP => self.op_pop(&mut current_call_frame),
                op if (Opcode::LOG0..=Opcode::LOG4).contains(&op) => {
                    self.op_log(&mut current_call_frame, op)
                }
                Opcode::MLOAD => self.op_mload(&mut current_call_frame),
                Opcode::MSTORE => self.op_mstore(&mut current_call_frame),
                Opcode::MSTORE8 => self.op_mstore8(&mut current_call_frame),
                Opcode::SLOAD => self.op_sload(&mut current_call_frame),
                Opcode::SSTORE => self.op_sstore(&mut current_call_frame),
                Opcode::MSIZE => self.op_msize(&mut current_call_frame),
                Opcode::GAS => self.op_gas(&mut current_call_frame),
                Opcode::MCOPY => self.op_mcopy(&mut current_call_frame),
                Opcode::CALL => self.op_call(&mut current_call_frame),
                Opcode::CALLCODE => self.op_callcode(&mut current_call_frame),
                Opcode::RETURN => self.op_return(&mut current_call_frame),
                Opcode::DELEGATECALL => self.op_delegatecall(&mut current_call_frame),
                Opcode::STATICCALL => self.op_staticcall(&mut current_call_frame),
                Opcode::CREATE => self.op_create(&mut current_call_frame),
                Opcode::CREATE2 => self.op_create2(&mut current_call_frame),
                Opcode::TLOAD => self.op_tload(&mut current_call_frame),
                Opcode::TSTORE => self.op_tstore(&mut current_call_frame),
                _ => Err(VMError::OpcodeNotFound),
            };

            match op_result {
                Ok(OpcodeSuccess::Continue) => {}
                Ok(OpcodeSuccess::Result(r)) => {
                    self.call_frames.push(current_call_frame.clone());
                    return Ok(Self::write_success_result(
                        current_call_frame.clone(),
                        r,
                        self.env.consumed_gas,
                        self.env.refunded_gas,
                    ));
                }
                Err(e) => {
                    self.call_frames.push(current_call_frame.clone());
                    return Ok(ExecutionResult::Halt {
                        reason: e,
                        gas_used: self.env.consumed_gas,
                    });
                }
            }
        }
    }

    pub fn transact(&mut self) -> Result<ResultAndState, VMError> {
        // let initial_gas_consumed = self.validate_transaction()?;
        let initial_gas_consumed = 0;
        let res = self.execute(initial_gas_consumed)?;
        self.get_result(res)
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
    ) -> Result<OpcodeSuccess, VMError> {
        // check balance
        if self.db.balance(&current_call_frame.msg_sender) < value {
            current_call_frame.stack.push(U256::from(REVERT_FOR_CALL))?;
            return Ok(OpcodeSuccess::Continue);
        }

        // transfer value
        // transfer(&current_call_frame.msg_sender, &address, value);

        let code_address_bytecode = self.db.get_account_bytecode(&code_address);
        if code_address_bytecode.is_empty() {
            // should stop
            current_call_frame
                .stack
                .push(U256::from(SUCCESS_FOR_CALL))?;
            return Ok(OpcodeSuccess::Result(ResultReason::Stop));
        }

        self.db.increment_account_nonce(&code_address);

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
            is_static,
            gas,
            current_call_frame.depth + 1,
        );

        current_call_frame.return_data_offset = Some(ret_offset);
        current_call_frame.return_data_size = Some(ret_size);

        self.call_frames.push(new_call_frame.clone());
        let result = self.execute(self.env.consumed_gas)?;

        match result {
            ExecutionResult::Success {
                logs, return_data, ..
            } => {
                current_call_frame.logs.extend(logs);
                current_call_frame
                    .memory
                    .store_bytes(ret_offset, &return_data);
                current_call_frame.returndata = return_data;
                current_call_frame
                    .stack
                    .push(U256::from(SUCCESS_FOR_CALL))?;
                Ok(OpcodeSuccess::Continue)
            }
            ExecutionResult::Revert {
                reason: _,
                gas_used,
                output,
            } => {
                current_call_frame.memory.store_bytes(ret_offset, &output);
                current_call_frame.returndata = output;
                current_call_frame.stack.push(U256::from(REVERT_FOR_CALL))?;
                current_call_frame.gas -= U256::from(gas_used);
                self.env.refunded_gas += gas_used;
                Ok(OpcodeSuccess::Continue)
            }
            ExecutionResult::Halt { reason, gas_used } => {
                current_call_frame
                    .stack
                    .push(U256::from(reason.clone() as u8))?;
                if U256::from(gas_used) > current_call_frame.gas {
                    current_call_frame.gas = U256::zero();
                } else {
                    current_call_frame.gas -= U256::from(gas_used);
                }
                Err(reason)
            } // WARNING: I commented this because I don't know when this should be executed.
              // Err(_) => {
              //     current_call_frame.stack.push(U256::from(HALT_FOR_CALL))?;
              // }
        }
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
    ) -> Result<OpcodeSuccess, VMError> {
        if code_size_in_memory > MAX_CODE_SIZE * 2 {
            current_call_frame
                .stack
                .push(U256::from(REVERT_FOR_CREATE))?;
            return Ok(OpcodeSuccess::Result(ResultReason::Revert));
        }
        if current_call_frame.is_static {
            current_call_frame
                .stack
                .push(U256::from(REVERT_FOR_CREATE))?;
            return Ok(OpcodeSuccess::Result(ResultReason::Revert));
        }

        let sender_account = self
            .db
            .accounts
            .get_mut(&current_call_frame.msg_sender)
            .unwrap();

        if sender_account.balance < value_in_wei_to_send {
            current_call_frame
                .stack
                .push(U256::from(REVERT_FOR_CREATE))?;
            return Ok(OpcodeSuccess::Result(ResultReason::Revert));
        }

        let Some(new_nonce) = sender_account.nonce.checked_add(1) else {
            current_call_frame
                .stack
                .push(U256::from(REVERT_FOR_CREATE))?;
            return Ok(OpcodeSuccess::Result(ResultReason::Revert));
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
            current_call_frame
                .stack
                .push(U256::from(REVERT_FOR_CREATE))?;
            return Ok(OpcodeSuccess::Result(ResultReason::Revert));
        }

        let new_account = Account::new(
            new_address,
            value_in_wei_to_send,
            code.clone(),
            0,
            Default::default(),
        );
        self.db.add_account(new_address, new_account);

        let mut gas = current_call_frame.gas;
        gas -= gas / 64; // 63/64 of the gas to the call
        current_call_frame.gas -= gas; // leaves 1/64  of the gas to current call frame

        current_call_frame
            .stack
            .push(address_to_word(new_address))?;

        self.generic_call(
            current_call_frame,
            gas,
            value_in_wei_to_send,
            current_call_frame.msg_sender,
            new_address,
            new_address,
            None,
            true,
            false,
            code_offset_in_memory,
            code_size_in_memory,
            code_offset_in_memory,
            code_size_in_memory,
        )
    }
}
