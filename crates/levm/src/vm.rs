use crate::opcodes::Opcode;
use bytes::Bytes;
use ethereum_types::U256;

#[derive(Debug, Clone, Default)]
pub struct VM {
    pub stack: Vec<U256>, // max 1024 in the future
    pub memory: Vec<u8>,
    pc: usize,
}

impl VM {
    pub fn execute(&mut self, mut bytecode: Bytes) {
        loop {
            match self.next_opcode(&mut bytecode).unwrap() {
                Opcode::STOP => break,
                Opcode::ADD => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    self.stack.push(a + b);
                }
                Opcode::PUSH32 => {
                    let next_32_bytes = bytecode.get(self.pc..self.pc + 32).unwrap();
                    let value_to_push = U256::from(next_32_bytes);
                    dbg!(value_to_push);
                    self.stack.push(value_to_push);
                    self.increment_pc_by(32);
                }
                Opcode::MLOAD => {
                    // spend_gas(3);
                    let offset = self.stack.pop().unwrap();
                    // resize if necessary
                    let value = self.memory[offset.as_usize()];
                    self.stack.push(value.into());
                }
                Opcode::MSTORE => {
                    let value = self.stack.pop().unwrap();
                    let address = self.stack.pop().unwrap();
                }
                Opcode::MSTORE8 => {
                    let value = self.stack.pop().unwrap();
                    let address = self.stack.pop().unwrap();
                }
                Opcode::MSIZE => {
                    let size = U256::zero(); // TODO: get size of memory
                    self.stack.push(size);
                }
                Opcode::MCOPY => {
                    let dest = self.stack.pop().unwrap();
                    let src = self.stack.pop().unwrap();
                    let size = self.stack.pop().unwrap();
                }
            }
        }
    }

    fn next_opcode(&mut self, opcodes: &mut Bytes) -> Option<Opcode> {
        let opcode = opcodes.get(self.pc).copied().map(Opcode::from);
        self.increment_pc();
        opcode
    }

    fn increment_pc_by(&mut self, count: usize) {
        self.pc += count;
    }

    fn increment_pc(&mut self) {
        self.increment_pc_by(1);
    }

    pub fn pc(&self) -> usize {
        self.pc
    }
}
