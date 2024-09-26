use crate::{memory::Memory, opcodes::Opcode, storage::TransientStorage};
use bytes::Bytes;
use ethereum_types::{Address, U256};

#[derive(Debug, Clone, Default)]
pub struct CallFrame {
    pub stack: Vec<U256>, // max 1024 in the future
    pub memory: Memory,
    pub transient_storage: TransientStorage, // TODO: this should actually go in the tx environment
    pub pc: usize,
    pub msg_sender: Address,
    pub callee: Address,
    pub bytecode: Bytes,
    pub delegate: Option<Address>,
    pub msg_value: U256,
}

impl CallFrame {
    pub fn next_opcode(&mut self) -> Option<Opcode> {
        let opcode = self.bytecode.get(self.pc).copied().map(Opcode::from);
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
}
