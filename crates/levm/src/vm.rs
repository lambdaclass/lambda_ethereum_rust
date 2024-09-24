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
                Opcode::LT => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    let result = if a < b { U256::one() } else { U256::zero() };
                    self.stack.push(result);
                }
                Opcode::GT => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    let result = if a > b { U256::one() } else { U256::zero() };
                    self.stack.push(result);
                }
                Opcode::SLT => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    let a_signed = if a.bit(255) {
                        -((!a + U256::one()).as_u128() as i128)
                    } else {
                        a.as_u128() as i128
                    };
                    let b_signed = if b.bit(255) {
                        -((!b + U256::one()).as_u128() as i128)
                    } else {
                        b.as_u128() as i128
                    };
                    let result = if a_signed < b_signed {
                        U256::one()
                    } else {
                        U256::zero()
                    };
                    self.stack.push(result);
                }
                Opcode::SGT => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    let a_signed = if a.bit(255) {
                        -((!a + U256::one()).as_u128() as i128)
                    } else {
                        a.as_u128() as i128
                    };
                    let b_signed = if b.bit(255) {
                        -((!b + U256::one()).as_u128() as i128)
                    } else {
                        b.as_u128() as i128
                    };
                    let result = if a_signed > b_signed {
                        U256::one()
                    } else {
                        U256::zero()
                    };
                    self.stack.push(result);
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

    pub fn pc(&self) -> usize {
        self.pc
    }
}
