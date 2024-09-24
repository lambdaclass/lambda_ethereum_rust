use crate::opcodes::Opcode;
use bytes::Bytes;
use ethereum_types::U256;

#[derive(Debug, Clone, Default)]
pub struct VM {
    pub stack: Vec<U256>, // max 1024 in the future
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
                op if (Opcode::PUSH1..Opcode::PUSH32).contains(&op) => {
                    let n_bytes = (op as u8) - (Opcode::PUSH1 as u8) + 1;
                    let next_n_bytes = bytecode.get(self.pc..self.pc + n_bytes as usize).unwrap();
                    let value_to_push = U256::from(next_n_bytes);
                    self.stack.push(value_to_push);
                    self.increment_pc_by(n_bytes as usize);
                }
                Opcode::PUSH32 => {
                    let next_32_bytes = bytecode.get(self.pc..self.pc + 32).unwrap();
                    let value_to_push = U256::from(next_32_bytes);
                    self.stack.push(value_to_push);
                    self.increment_pc_by(32);
                }
                _ => unimplemented!(),
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

#[cfg(test)]
mod tests {
    use crate::operations::Operation;

    use super::*;

    #[test]
    fn push1_ok() {
        let mut vm = VM::default();

        let to_push = U256::from_big_endian(&[0xff]);

        let operations = [Operation::Push((1, to_push)), Operation::Stop];
        let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

        vm.execute(bytecode);

        assert_eq!(vm.stack[0], to_push);
        assert_eq!(vm.pc, 3);
    }

    #[test]
    fn push5_ok() {
        let mut vm = VM::default();

        let to_push = U256::from_big_endian(&[0xff, 0xff, 0xff, 0xff, 0xff]);

        let operations = [Operation::Push((5, to_push)), Operation::Stop];
        let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

        vm.execute(bytecode);

        assert_eq!(vm.stack[0], to_push);
        assert_eq!(vm.pc, 7);
    }
}
