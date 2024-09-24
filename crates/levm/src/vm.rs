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
                Opcode::MUL => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    self.stack.push(a * b);
                }
                Opcode::SUB => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    self.stack.push(a - b);
                }
                Opcode::DIV => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    if b.is_zero() {
                        self.stack.push(U256::zero());
                        continue;
                    }

                    self.stack.push(a / b);
                }
                Opcode::SDIV => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    if b.is_zero() {
                        self.stack.push(U256::zero());
                        continue;
                    }
                    let a_is_negative = a >> 255 == U256::one();
                    let b_is_negative = b >> 255 == U256::one();

                    let a = if a_is_negative { !a + U256::one() } else { a };
                    let b = if b_is_negative { !b + U256::one() } else { b };
                    let result = a / b;

                    let is_result_negative = a_is_negative ^ b_is_negative;

                    let result = if is_result_negative {
                        !result + U256::one()
                    } else {
                        result
                    };

                    self.stack.push(result);
                }
                Opcode::MOD => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    if b.is_zero() {
                        self.stack.push(U256::zero());
                        continue;
                    }

                    self.stack.push(a % b);
                }
                Opcode::SMOD => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    if b.is_zero() {
                        self.stack.push(U256::zero());
                        continue;
                    }

                    let a_is_negative = a >> 255 == U256::one();
                    let b_is_negative = b >> 255 == U256::one();
                    let a = if a_is_negative { !a + U256::one() } else { a };
                    let b = if b_is_negative { !b + U256::one() } else { b };
                    let result = a % b;
                    let result_is_negative = a_is_negative ^ b_is_negative;
                    let result = if result_is_negative {
                        !result + U256::one()
                    } else {
                        result
                    };

                    self.stack.push(result);
                }
                Opcode::ADDMOD => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    let n = self.stack.pop().unwrap();
                    if n.is_zero() {
                        self.stack.push(U256::zero());
                        continue;
                    }

                    self.stack.push((a + b) % n);
                }
                Opcode::MULMOD => {
                    let a = self.stack.pop().unwrap();
                    let b = self.stack.pop().unwrap();
                    let n = self.stack.pop().unwrap();
                    if n.is_zero() {
                        self.stack.push(U256::zero());
                        continue;
                    }

                    self.stack.push((a * b) % n);
                }
                Opcode::EXP => {
                    let base = self.stack.pop().unwrap();
                    let exponent = self.stack.pop().unwrap();
                    self.stack.push(base.pow(exponent));
                }
                Opcode::SIGNEXTEND => {
                    let byte_size = self.stack.pop().unwrap();
                    let value_to_extend = self.stack.pop().unwrap();

                    let bits_per_byte = U256::from(8);
                    let sign_bit_position_on_byte = 7;
                    let max_byte_size = 31;

                    let byte_size = byte_size.min(U256::from(max_byte_size));
                    let sign_bit_index = bits_per_byte * byte_size + sign_bit_position_on_byte;
                    let is_negative = value_to_extend.bit(sign_bit_index.as_usize());
                    let sign_bit_mask = (U256::one() << sign_bit_index) - U256::one();
                    let result = if is_negative {
                        value_to_extend.saturating_add(!sign_bit_mask)
                    } else {
                        value_to_extend & sign_bit_mask
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
