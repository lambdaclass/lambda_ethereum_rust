use crate::opcodes::Opcode;
use bytes::Bytes;
use ethereum_types::U256;

#[derive(Debug, Clone)]
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
                Opcode::PUSH32 => {
                    let next_32_bytes = bytecode.get(self.pc..self.pc + 32).unwrap();
                    let value_to_push = U256::from(next_32_bytes);
                    dbg!(value_to_push);
                    self.stack.push(value_to_push);
                    self.increment_pc_by(32);
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
}

#[cfg(test)]
mod tests {
    use crate::operations::Operation;

    use super::*;

    #[test]
    fn test() {
        let mut vm = VM {
            stack: vec![],
            pc: 0,
        };

        let operations = [
            Operation::Push32(U256::one()),
            Operation::Push32(U256::zero()),
            Operation::Add,
            Operation::Stop,
        ];

        let bytecode = operations
            .iter()
            .flat_map(Operation::to_bytecode)
            .collect::<Bytes>();

        vm.execute(bytecode);

        println!("{vm:?}");
    }
}
