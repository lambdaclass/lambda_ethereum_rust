use std::{
    cmp, collections::{HashMap, HashSet}, str::FromStr, u64
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

    pub fn with_address(mut self, address: Address) -> Self {
        self.address = address;
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

    pub fn get_account_bytecode(&self, address: &Address) -> Bytes {
        self.accounts
            .get(address)
            .map_or(Bytes::new(), |acc| acc.bytecode.clone())
    }

    pub fn balance(&mut self, address: &Address) -> U256 {
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

    /// Returns the account associated with the given address.
    /// If the account does not exist in the Db, it creates a new one with the given address.
    pub fn get_account(&mut self, address: &Address) -> Result<&Account, VMError> {
        if self.accounts.contains_key(address) {
            return Ok(self.accounts.get(address).unwrap());
        }

        let new_account = Account {
            address: *address,
            ..Default::default()
        };

        self.accounts.insert(*address, new_account);

        Ok(self.accounts.get(address).unwrap())
    }
}

#[derive(Debug, Clone, Default)]
// TODO: https://github.com/lambdaclass/ethereum_rust/issues/604
pub struct Substate {
    pub warm_addresses: HashSet<Address>,
    pub self_destruct_set: HashSet<Address>,
    pub created_contracts: HashSet<Address>,
}

#[derive(Debug, Default, Clone)]
pub struct Environment {
    /// The sender address of the transaction that originated
    /// this execution.
    pub origin: Address,
    pub consumed_gas: u64,
    refunded_gas: u64,
    /// The block header of the present block.
    pub block: BlockEnv,
    pub tx_env: TxEnv,
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

pub fn word_to_address(word: U256) -> Address {
    let mut bytes = [0u8; 32];
    word.to_big_endian(&mut bytes);
    Address::from_slice(&bytes[12..])
}

impl VM {
    pub fn new(tx_env: TxEnv, block_env: BlockEnv, mut db: Db) -> Self {
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
            tx_env.data.clone(),
            false,
            u64::MAX,
            0,
            0,
        );

        let env = Environment {
            block: block_env,
            consumed_gas: TX_BASE_COST,
            origin: tx_env.msg_sender,
            refunded_gas: 0,
            tx_env,
        };
        let mut accrued_substate = Substate::default();

        Self::add_coinbase_to_db(&env.block, &mut db, &mut accrued_substate);

        Self {
            call_frames: vec![initial_call_frame],
            db,
            env,
            accrued_substate,
        }
    }

    fn add_coinbase_to_db(block: &BlockEnv, db: &mut Db, accrued_substate: &mut Substate) {
        let coinbase = block.coinbase;
        let account = Account::new(coinbase, U256::zero(), Bytes::new(), 0, Default::default());
        db.add_account(coinbase, account);
        accrued_substate.warm_addresses.insert(coinbase);
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
            ResultReason::SelfDestruct => todo!(),
        }
    }

    pub fn execute(&mut self) -> ExecutionResult {
        let mut current_call_frame = self
            .call_frames
            .pop()
            .expect("Fatal Error: This should not happen"); // if this happens during execution, we are cooked 💀
        loop {
            let opcode = current_call_frame.next_opcode().unwrap_or(Opcode::STOP);
            let op_result: Result<OpcodeSuccess, VMError> = match opcode {
                Opcode::STOP => Ok(OpcodeSuccess::Result(ResultReason::Stop)),
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
                Opcode::JUMPDEST => self.op_jumpdest(&mut current_call_frame),
                Opcode::PC => self.op_pc(&mut current_call_frame),
                Opcode::BLOCKHASH => self.op_blockhash(&mut current_call_frame),
                Opcode::COINBASE => self.op_coinbase(&mut current_call_frame),
                Opcode::TIMESTAMP => self.op_timestamp(&mut current_call_frame),
                Opcode::NUMBER => self.op_number(&mut current_call_frame),
                Opcode::PREVRANDAO => self.op_prevrandao(&mut current_call_frame),
                Opcode::GASLIMIT => self.op_gaslimit(&mut current_call_frame),
                Opcode::CHAINID => self.op_chainid(&mut current_call_frame),
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
                Opcode::SELFBALANCE => self.op_selfbalance(&mut current_call_frame),
                Opcode::ADDRESS => self.op_address(&mut current_call_frame),
                Opcode::ORIGIN => self.op_origin(&mut current_call_frame),
                Opcode::BALANCE => self.op_balance(&mut current_call_frame),
                Opcode::CALLER => self.op_caller(&mut current_call_frame),
                Opcode::CALLVALUE => self.op_callvalue(&mut current_call_frame),
                Opcode::CODECOPY => self.op_codecopy(&mut current_call_frame),
                Opcode::CODESIZE => self.op_codesize(&mut current_call_frame),
                Opcode::GASPRICE => self.op_gasprice(&mut current_call_frame),
                Opcode::REVERT => self.op_revert(&mut current_call_frame),
                Opcode::INVALID => self.op_invalid(),
                Opcode::SELFDESTRUCT => self.op_selfdestruct(&mut current_call_frame),
                Opcode::EXTCODESIZE => self.op_extcodesize(&mut current_call_frame),
                Opcode::EXTCODECOPY => self.op_extcodecopy(&mut current_call_frame),
                Opcode::EXTCODEHASH => self.op_extcodehash(&mut current_call_frame),
                _ => Err(VMError::OpcodeNotFound),
            };

            match op_result {
                Ok(OpcodeSuccess::Continue) => {}
                Ok(OpcodeSuccess::Result(r)) => {
                    self.call_frames.push(current_call_frame.clone());
                    return Self::write_success_result(
                        current_call_frame.clone(),
                        r,
                        self.env.consumed_gas,
                        self.env.refunded_gas,
                    );
                }
                Err(e) => {
                    // Any error triggers a Revert, even Revert Opcode is considered an execution error (the only difference is in gas consumption and output).
                    self.call_frames.push(current_call_frame.clone());

                    let (unused_gas, output) = if e == VMError::RevertOpcode {
                        (current_call_frame.gas_limit - current_call_frame.gas_used, current_call_frame.returndata.clone())
                    }
                    else{
                        (0, Bytes::new())
                    };
    
                    return ExecutionResult::Revert { 
                        reason: e,
                        unused_gas,
                        output 
                    };
                }
            }
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
                unused_gas: 0, //TODO
                reason: res.reason(),
            },
            ExitStatusCode::Error | ExitStatusCode::Default => ExecutionResult::Revert { 
                reason: VMError::FatalError, //TODO, change this  
                unused_gas: 0, 
                output: Bytes::new() 
            }
        };

        // TODO: Check if this is ok
        let state = self.db.into_state();

        Ok(ResultAndState { result, state })
    }

    pub fn transact(&mut self) -> Result<ResultAndState, VMError> {
        let account = self.db.accounts.get(&self.env.tx_env.msg_sender).unwrap();

        let initial_gas = match self
            .env
            .tx_env
            .validate_transaction(account, &self.env.block)
        {
            Ok(gas) => gas,
            Err(e) => {
                return self.get_result(ExecutionResult::Revert {
                    reason: e,
                    unused_gas: 0, //TODO
                    output: Bytes::new(),
                })
            }
        };

        self.env.consumed_gas = initial_gas;

        let res = self.execute();
        self.get_result(res)
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

        
        let gas_limit = cmp::min(gas.as_u64(), (current_call_frame.gas_limit - current_call_frame.gas_used) * (63 / 64)); // Gas limit for new callframe.
        let new_call_frame = CallFrame::new(
            msg_sender,
            to,
            code_address,
            delegate,
            code_address_bytecode,
            value,
            calldata,
            is_static,
            gas_limit,
            0,
            current_call_frame.depth + 1,
        );

        self.increase_consumed_gas(current_call_frame, gas_limit)?; // Gas sent to the new callframe is "spent" by the caller. (In some cases it is returned back afterwards)

        current_call_frame.return_data_offset = Some(ret_offset);
        current_call_frame.return_data_size = Some(ret_size);


        let backup_vm = self.clone(); // Clone VM for restoring it's state if revert happens

        self.call_frames.push(new_call_frame.clone());
        let result = self.execute();

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
                // Here we should return unused gas to the caller
            }
            ExecutionResult::Revert {
                reason,
                unused_gas,
                output,
            } => {
                // Restore the VM to the state before the call
                *self = backup_vm;

                // 1. Pushing 0 to stack
                current_call_frame.stack.push(U256::from(REVERT_FOR_CALL))?;

                // If reverted by Revert opcode we should return unused gas to the caller and also store the return data in memory
                if reason == VMError::RevertOpcode {
                    // 2. Return the unused gas to the caller (If not exceptional halt)
                    self.decrease_consumed_gas(current_call_frame, unused_gas);

                    // 3. Storing in memory the return data of the sub-context (in offset: ret_offset, with size: ret_size)
                    current_call_frame.memory.store_n_bytes(ret_offset, &output, ret_size);
                };

                // 4. Reverting gas refunds (triggered by SSTORE)
                // gas_refunds are not implemented yet so this is going to stay as a comment for now.
            }
        }

        Ok(OpcodeSuccess::Continue)
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
            return Err(VMError::RevertOpcode);
        }
        if current_call_frame.is_static {
            current_call_frame
                .stack
                .push(U256::from(REVERT_FOR_CREATE))?;
            return Err(VMError::RevertOpcode);
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
            return Err(VMError::RevertOpcode);
        }

        let Some(new_nonce) = sender_account.nonce.checked_add(1) else {
            current_call_frame
                .stack
                .push(U256::from(REVERT_FOR_CREATE))?;
            return Err(VMError::RevertOpcode);
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
            return Err(VMError::RevertOpcode);
        }

        let new_account = Account::new(
            new_address,
            value_in_wei_to_send,
            code.clone(),
            0,
            Default::default(),
        );
        self.db.add_account(new_address, new_account);

        let gas: U256 = ((current_call_frame.gas_limit - current_call_frame.gas_used) / 64).into();

        current_call_frame
            .stack
            .push(address_to_word(new_address))?;


        // Add to substate created contracts in transaction (for SelfDestruct)
        self.accrued_substate.created_contracts.insert(new_address);

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

    /// Increases gas consumption of CallFrame and Environment, returning an error if the callframe gas limit is reached.
    pub fn increase_consumed_gas(&mut self, current_call_frame: &mut CallFrame, gas: u64) -> Result<(), VMError>{
        if current_call_frame.gas_used + gas > current_call_frame.gas_limit {
            return Err(VMError::OutOfGas);
        }
        current_call_frame.gas_used += gas;
        self.env.consumed_gas += gas;
        Ok(())
    }

    /// Decreases gas consumption of CallFrame and Environment.
    pub fn decrease_consumed_gas(&mut self, current_call_frame: &mut CallFrame, gas: u64) {
        current_call_frame.gas_used -= gas;
        self.env.consumed_gas -= gas;
    }
}
