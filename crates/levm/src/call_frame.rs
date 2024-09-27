use crate::{
    memory::Memory,
    opcodes::Opcode,
    primitives::{Address, Bytes, U256},
};

#[derive(Debug, Clone, Default)]
pub struct CallFrame {
    pub gas: U256,
    pub pc: usize,
    pub msg_sender: Address,
    pub callee: Address,
    pub bytecode: Bytes,
    pub delegate: Option<Address>,
    pub msg_value: U256,
    pub stack: Vec<U256>, // max 1024 in the future
    pub memory: Memory,
    pub calldata: Bytes,
    pub return_data: Bytes,
    // where to store return data of subcall
    pub return_data_offset: Option<usize>,
    pub return_data_size: Option<usize>,
}

impl CallFrame {
    pub fn new(bytecode: Bytes) -> Self {
        Self {
            bytecode,
            return_data_offset: None,
            return_data_size: None,
            ..Default::default()
        }
    }

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
