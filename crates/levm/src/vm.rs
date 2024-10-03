use std::{collections::HashMap, str::FromStr};

use crate::{
    block::{BlockEnv, LAST_AVAILABLE_BLOCK_LIMIT},
    call_frame::CallFrame,
    constants::{HALT_FOR_CALL, REVERT_FOR_CALL, SUCCESS_FOR_CALL, SUCCESS_FOR_RETURN},
    opcodes::Opcode,
    vm_result::{ExecutionResult, ResultReason, VMError},
};
use bytes::Bytes;
use ethereum_types::{Address, H256, U256};

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
                    self.op_add(&mut current_call_frame)?;
                }
                Opcode::MUL => {
                    self.op_mul(&mut current_call_frame)?;
                }
                Opcode::SUB => {
                    self.op_sub(&mut current_call_frame)?;
                }
                Opcode::DIV => {
                    self.op_div(&mut current_call_frame)?;
                }
                Opcode::SDIV => {
                    self.op_sdiv(&mut current_call_frame)?;
                }
                Opcode::MOD => {
                    self.op_modulus(&mut current_call_frame)?;
                }
                Opcode::SMOD => {
                    self.op_smod(&mut current_call_frame)?;
                }
                Opcode::ADDMOD => {
                    self.op_addmod(&mut current_call_frame)?;
                }
                Opcode::MULMOD => {
                    self.op_mulmod(&mut current_call_frame)?;
                }
                Opcode::EXP => {
                    self.op_exp(&mut current_call_frame)?;
                }
                Opcode::SIGNEXTEND => {
                    self.op_signextend(&mut current_call_frame)?;
                }
                Opcode::LT => {
                    self.op_lt(&mut current_call_frame)?;
                }
                Opcode::GT => {
                    self.op_gt(&mut current_call_frame)?;
                }
                Opcode::SLT => {
                    self.op_slt(&mut current_call_frame)?;
                }
                Opcode::SGT => {
                    self.op_sgt(&mut current_call_frame)?;
                }
                Opcode::EQ => {
                    self.op_eq(&mut current_call_frame)?;
                }
                Opcode::ISZERO => {
                    self.op_iszero(&mut current_call_frame)?;
                }
                Opcode::KECCAK256 => {
                    self.op_keccak256(&mut current_call_frame)?;
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
                    self.op_blockhash(&mut current_call_frame, &block_env)?;
                }
                Opcode::COINBASE => {
                    self.op_coinbase(&mut current_call_frame, &block_env)?;
                }
                Opcode::TIMESTAMP => {
                    self.op_timestamp(&mut current_call_frame, &block_env)?;
                }
                Opcode::NUMBER => {
                    self.op_number(&mut current_call_frame, &block_env)?;
                }
                Opcode::PREVRANDAO => {
                    self.op_prevrandao(&mut current_call_frame, &block_env)?;
                }
                Opcode::GASLIMIT => {
                    self.op_gaslimit(&mut current_call_frame, &block_env)?;
                }
                Opcode::CHAINID => {
                    self.op_chainid(&mut current_call_frame, &block_env)?;
                }
                Opcode::SELFBALANCE => {
                    self.op_selfbalance(&mut current_call_frame, &block_env)?;
                }
                Opcode::BASEFEE => {
                    self.op_basefee(&mut current_call_frame, &block_env)?;
                }
                Opcode::BLOBHASH => {
                    self.op_blobhash(&mut current_call_frame, &block_env)?;
                }
                Opcode::BLOBBASEFEE => {
                    self.op_blobbasefee(&mut current_call_frame, &block_env)?;
                }
                Opcode::PUSH0 => {
                    current_call_frame.stack.push(U256::zero())?;
                }
                // PUSHn
                op if (Opcode::PUSH1..Opcode::PUSH32).contains(&op) => {
                    self.op_push(&mut current_call_frame, op)?;
                }
                Opcode::PUSH32 => {
                    self.op_push(&mut current_call_frame, Opcode::PUSH32)?; // This opcode is removed in another branch
                }
                Opcode::AND => {
                    self.op_and(&mut current_call_frame)?;
                }
                Opcode::OR => {
                    self.op_or(&mut current_call_frame)?;
                }
                Opcode::XOR => {
                    self.op_xor(&mut current_call_frame)?;
                }
                Opcode::NOT => {
                    self.op_not(&mut current_call_frame)?;
                }
                Opcode::BYTE => {
                    self.op_byte(&mut current_call_frame)?;
                }
                Opcode::SHL => {
                    self.op_shl(&mut current_call_frame)?;
                }
                Opcode::SHR => {
                    self.op_shr(&mut current_call_frame)?;
                }
                Opcode::SAR => {
                    self.op_sar(&mut current_call_frame)?;
                }
                // DUPn
                op if (Opcode::DUP1..=Opcode::DUP16).contains(&op) => {
                    self.op_dup(&mut current_call_frame, op)?;
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
                    self.op_log(&mut current_call_frame, op)?;
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
