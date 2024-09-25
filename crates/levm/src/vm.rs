use crate::opcodes::Opcode;
use bytes::Bytes;
use ethereum_types::U256;

#[derive(Debug, Clone, Default)]
pub struct VM {
    pub stack: Vec<U256>, // max 1024 in the future
    pub memory: Memory,
    pub pc: usize,
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
                Opcode::PUSH0 => {
                    self.stack.push(U256::zero());
                }
                // PUSHn
                op if (Opcode::PUSH1..Opcode::PUSH32).contains(&op) => {
                    let n_bytes = (op as u8) - (Opcode::PUSH1 as u8) + 1;
                    let next_n_bytes = bytecode
                        .get(self.pc..self.pc + n_bytes as usize)
                        .expect("invalid bytecode");
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
                // DUPn
                op if (Opcode::DUP1..=Opcode::DUP16).contains(&op) => {
                    let depth = (op as u8) - (Opcode::DUP1 as u8) + 1;
                    assert!(
                        self.stack.len().ge(&(depth as usize)),
                        "stack underflow: not enough values on the stack"
                    );
                    let value_at_depth = self.stack.get(self.stack.len() - depth as usize).unwrap();
                    self.stack.push(*value_at_depth);
                }
                // SWAPn
                op if (Opcode::SWAP1..=Opcode::SWAP16).contains(&op) => {
                    let depth = (op as u8) - (Opcode::SWAP1 as u8) + 1;
                    assert!(
                        self.stack.len().ge(&(depth as usize)),
                        "stack underflow: not enough values on the stack"
                    );
                    let stack_top_index = self.stack.len();
                    let to_swap_index = stack_top_index.checked_sub(depth as usize).unwrap();
                    self.stack.swap(stack_top_index - 1, to_swap_index - 1);
                }
                _ => unimplemented!(),
                Opcode::MLOAD => {
                    // spend_gas(3);
                    let offset = self.stack.pop().unwrap().try_into().unwrap();
                    let value = self.memory.load(offset);
                    self.stack.push(value);
                }
                Opcode::MSTORE => {
                    // spend_gas(3);
                    let offset = self.stack.pop().unwrap().try_into().unwrap();
                    let value = self.stack.pop().unwrap();
                    let mut value_bytes = [0u8; 32];
                    value.to_big_endian(&mut value_bytes);

                    self.memory.store_bytes(offset, &value_bytes);
                }
                Opcode::MSTORE8 => {
                    // spend_gas(3);
                    let offset = self.stack.pop().unwrap().try_into().unwrap();
                    let value = self.stack.pop().unwrap();
                    let mut value_bytes = [0u8; 32];
                    value.to_big_endian(&mut value_bytes);

                    self.memory
                        .store_bytes(offset, value_bytes[31..32].as_ref());
                }
                Opcode::MSIZE => {
                    // spend_gas(2);
                    self.stack.push(self.memory.size());
                }
                Opcode::MCOPY => {
                    // spend_gas(3) + dynamic gas
                    let dest_offset = self.stack.pop().unwrap().try_into().unwrap();
                    let src_offset = self.stack.pop().unwrap().try_into().unwrap();
                    let size = self.stack.pop().unwrap().try_into().unwrap();
                    if size == 0 {
                        continue;
                    }

                    self.memory.copy(src_offset, dest_offset, size);
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

#[derive(Debug, Clone, Default)]
pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    fn resize(&mut self, offset: usize) {
        if offset.next_multiple_of(32) > self.data.len() {
            self.data.resize(offset.next_multiple_of(32), 0);
        }
    }

    pub fn load(&mut self, offset: usize) -> U256 {
        self.resize(offset + 32);
        let value_bytes: [u8; 32] = self
            .data
            .get(offset..offset + 32)
            .unwrap()
            .try_into()
            .unwrap();
        U256::from(value_bytes)
    }

    pub fn store_bytes(&mut self, offset: usize, value: &[u8]) {
        let len = value.len();
        self.resize(offset + len);
        self.data
            .splice(offset..offset + len, value.iter().copied());
    }

    pub fn size(&self) -> U256 {
        U256::from(self.data.len())
    }

    pub fn copy(&mut self, src_offset: usize, dest_offset: usize, size: usize) {
        let max_size = std::cmp::max(src_offset + size, dest_offset + size);
        self.resize(max_size);
        let mut temp = vec![0u8; size];

        temp.copy_from_slice(&self.data[src_offset..src_offset + size]);

        self.data[dest_offset..dest_offset + size].copy_from_slice(&temp);
    }
}

#[cfg(test)]
mod tests {
    use crate::operations::Operation;

    use super::*;

    #[test]
    fn push0_ok() {
        let mut vm = VM::default();

        let operations = [Operation::Push0, Operation::Stop];
        let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

        vm.execute(bytecode);

        assert_eq!(vm.stack[0], U256::zero());
        assert_eq!(vm.pc, 2);
    }

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

    #[test]
    fn push31_ok() {
        let mut vm = VM::default();

        let to_push = U256::from_big_endian(&[0xff; 31]);

        let operations = [Operation::Push((31, to_push)), Operation::Stop];
        let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

        vm.execute(bytecode);

        assert_eq!(vm.stack[0], to_push);
        assert_eq!(vm.pc, 33);
    }

    #[test]
    fn push32_ok() {
        let mut vm = VM::default();

        let to_push = U256::from_big_endian(&[0xff; 32]);

        let operations = [Operation::Push32(to_push), Operation::Stop];
        let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

        vm.execute(bytecode);

        assert_eq!(vm.stack[0], to_push);
        assert_eq!(vm.pc, 34);
    }

    #[test]
    fn dup1_ok() {
        let mut vm = VM::default();
        let value = U256::one();

        let operations = [
            Operation::Push((1, value)),
            Operation::Dup(1),
            Operation::Stop,
        ];
        let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

        vm.execute(bytecode);

        assert_eq!(vm.stack.len(), 2);
        assert_eq!(vm.pc, 4);
        assert_eq!(vm.stack[vm.stack.len() - 1], value);
        assert_eq!(vm.stack[vm.stack.len() - 2], value);
    }

    #[test]
    fn dup16_ok() {
        let mut vm = VM::default();
        let value = U256::one();

        let mut operations = vec![Operation::Push((1, value))];
        operations.extend(vec![Operation::Push0; 15]);
        operations.extend(vec![Operation::Dup(16), Operation::Stop]);

        let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

        vm.execute(bytecode);

        assert_eq!(vm.stack.len(), 17);
        assert_eq!(vm.pc, 19);
        assert_eq!(vm.stack[vm.stack.len() - 1], value);
        assert_eq!(vm.stack[vm.stack.len() - 17], value);
    }

    #[test]
    #[should_panic]
    fn dup_panics_if_stack_underflow() {
        let mut vm = VM::default();

        let operations = vec![Operation::Dup(5), Operation::Stop];
        let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

        vm.execute(bytecode);
    }

    #[test]
    fn swap1_ok() {
        let mut vm = VM::default();
        let bottom = U256::from_big_endian(&[0xff]);
        let top = U256::from_big_endian(&[0xee]);

        let operations = [
            Operation::Push((1, bottom)),
            Operation::Push((1, top)),
            Operation::Swap(1),
            Operation::Stop,
        ];
        let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

        vm.execute(bytecode);

        assert_eq!(vm.stack.len(), 2);
        assert_eq!(vm.pc, 6);
        assert_eq!(vm.stack[0], top);
        assert_eq!(vm.stack[1], bottom);
    }

    #[test]
    fn swap16_ok() {
        let mut vm = VM::default();
        let bottom = U256::from_big_endian(&[0xff]);
        let top = U256::from_big_endian(&[0xee]);

        let mut operations = vec![Operation::Push((1, bottom))];
        operations.extend(vec![Operation::Push0; 15]);
        operations.extend(vec![Operation::Push((1, top))]);
        operations.extend(vec![Operation::Swap(16), Operation::Stop]);

        let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

        vm.execute(bytecode);

        assert_eq!(vm.stack.len(), 17);
        assert_eq!(vm.pc, 21);
        assert_eq!(vm.stack[vm.stack.len() - 1], bottom);
        assert_eq!(vm.stack[vm.stack.len() - 1 - 16], top);
    }

    #[test]
    #[should_panic]
    fn swap_panics_if_stack_underflow() {
        let mut vm = VM::default();

        let operations = vec![Operation::Swap(5), Operation::Stop];
        let bytecode = operations.iter().flat_map(Operation::to_bytecode).collect();

        vm.execute(bytecode);
    }
}
