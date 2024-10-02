use ethereum_types::H32;

use crate::{
    memory::Memory,
    opcodes::Opcode,
    primitives::{Address, Bytes, U256},
};
use std::collections::HashMap;

/// [EIP-1153]: https://eips.ethereum.org/EIPS/eip-1153#reference-implementation
pub type TransientStorage = HashMap<(Address, U256), U256>;

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
/// Data record produced during the execution of a transaction.
pub struct Log {
    pub address: Address,
    pub topics: Vec<H32>,
    pub data: Bytes,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CallFrame {
    pub gas: U256,
    pub pc: usize,
    pub msg_sender: Address,
    pub to: Address,
    pub code_address: Address,
    pub delegate: Option<Address>,
    pub bytecode: Bytes,
    pub msg_value: U256,
    pub stack: Vec<U256>, // max 1024 in the future
    pub memory: Memory,
    pub calldata: Bytes,
    pub returndata: Bytes,
    // where to store return data of subcall
    pub return_data_offset: Option<usize>,
    pub return_data_size: Option<usize>,
    pub is_static: bool,
    pub transient_storage: TransientStorage,
    pub logs: Vec<Log>,
}

impl CallFrame {
    pub fn new_from_bytecode(bytecode: Bytes) -> Self {
        Self {
            bytecode,
            ..Default::default()
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        gas: U256,
        msg_sender: Address,
        to: Address,
        code_address: Address,
        delegate: Option<Address>,
        bytecode: Bytes,
        msg_value: U256,
        calldata: Bytes,
        is_static: bool,
    ) -> Self {
        Self {
            gas,
            msg_sender,
            to,
            code_address,
            delegate,
            bytecode,
            msg_value,
            calldata,
            is_static,
            ..Default::default()
        }
    }

    pub fn next_opcode(&mut self) -> Option<Opcode> {
        let opcode = self.opcode_at(self.pc);
        self.increment_pc();
        opcode
    }

    pub fn increment_pc_by(&mut self, count: usize) {
        self.pc += count;
    }

    pub fn increment_pc(&mut self) {
        self.increment_pc_by(1);
    }

    pub fn pc(&self) -> usize {
        self.pc
    }

    pub fn jump(&mut self, jump_address: U256) {
        if !self.valid_jump(jump_address) {
            // Should be a halt when we implement it
            panic!("Invalid jump");
        }
        self.pc = jump_address.as_usize() + 1;
    }

    fn valid_jump(&self, jump_address: U256) -> bool {
        self.opcode_at(jump_address.as_usize())
            .map(|opcode| opcode.eq(&Opcode::JUMPDEST))
            .is_some_and(|is_jumpdest| is_jumpdest)
    }

    fn opcode_at(&self, offset: usize) -> Option<Opcode> {
        self.bytecode.get(offset).copied().map(Opcode::from)
    }
}
