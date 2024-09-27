use crate::{memory::Memory, opcodes::Opcode};
use bytes::Bytes;
use ethereum_types::{Address, H32, U256};

#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
/// Data record produced during the execution of a transaction.
pub struct Log {
    pub address: Address,
    pub topics: Vec<H32>,
    pub data: Bytes,
}

#[derive(Debug, Clone, Default)]
pub struct CallFrame {
    pub stack: Vec<U256>, // max 1024 in the future
    pub memory: Memory,
    pub pc: usize,
    pub msg_sender: Address,
    pub callee: Address,
    pub bytecode: Bytes,
    pub delegate: Option<Address>,
    pub msg_value: U256,
    pub logs: Vec<Log>,
    pub is_static: bool,
}

impl CallFrame {
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
