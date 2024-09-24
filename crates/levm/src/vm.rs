use crate::opcodes::Opcode;
use bytes::Bytes;
use ethereum_types::U256;

#[derive(Debug, Clone, Default)]
pub struct VM {
    pub stack: Vec<U256>, // max 1024 in the future
    pub memory: Memory,
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
                    self.memory.resize(offset.as_usize());

                    let value = self.memory.load(offset.as_usize());
                    self.stack.push(value);
                }
                Opcode::MSTORE => {
                    // spend_gas(3);
                    let offset = self.stack.pop().unwrap().as_usize();
                    let value = self.stack.pop().unwrap();
                    let mut value_bytes = [0u8; 32];
                    value.to_big_endian(&mut value_bytes);
                    self.memory.resize(offset);

                    self.memory.store_bytes(offset, &value_bytes);
                }
                Opcode::MSTORE8 => {
                    // spend_gas(3);
                    let offset = self.stack.pop().unwrap().as_usize();
                    let value = self.stack.pop().unwrap();
                    let mut value_bytes = [0u8; 32];
                    value.to_big_endian(&mut value_bytes);
                    self.memory.resize(offset);

                    self.memory
                        .store_bytes(offset, value_bytes[31..32].as_ref());
                }
                Opcode::MSIZE => {
                    // spend_gas(2);
                    self.stack.push(self.memory.size());
                }
                Opcode::MCOPY => {
                    // spend_gas(3) + dynamic gas
                    let dest_offset = self.stack.pop().unwrap().as_usize();
                    let src_offset = self.stack.pop().unwrap().as_usize();
                    let size = self.stack.pop().unwrap().as_usize();
                    if size == 0 {
                        continue;
                    }

                    let max_size = std::cmp::max(src_offset + size, dest_offset + size);
                    self.memory.resize(max_size);

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

    pub fn resize(&mut self, offset: usize) {
        if offset + 32 > self.data.len() {
            self.data.resize(offset + 32, 0);
        }
    }

    pub fn load(&self, offset: usize) -> U256 {
        let value_bytes: [u8; 32] = self.data.get(offset..offset + 32).unwrap().try_into().unwrap();
        U256::from(value_bytes)
    }

    pub fn store_bytes(&mut self, offset: usize, value: &[u8]) {
        self.data
            .splice(offset..offset + value.len(), value.iter().copied());
    }

    pub fn size(&self) -> U256 {
        U256::from(self.data.len())
    }

    pub fn copy(&mut self, src_offset: usize, dest_offset: usize, size: usize) {
        let mut temp = vec![0u8; size];

        temp.copy_from_slice(&self.data[src_offset..src_offset + size]);

        self.data[dest_offset..dest_offset + size].copy_from_slice(&temp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mstore() {
        let mut vm = VM::default();

        vm.stack.push(U256::from(0x33333)); // value
        vm.stack.push(U256::from(0)); // offset

        vm.execute(Bytes::from(vec![Opcode::MSTORE as u8, Opcode::STOP as u8]));

        let stored_value = vm.memory.load(0);

        assert_eq!(stored_value, U256::from(0x33333));
    }

    #[test]
    fn test_mstore8() {
        let mut vm = VM::default();

        vm.stack.push(U256::from(0xAB)); // value
        vm.stack.push(U256::from(0)); // offset

        vm.execute(Bytes::from(vec![Opcode::MSTORE8 as u8, Opcode::STOP as u8]));

        let stored_value = vm.memory.load(0);

        let mut value_bytes = [0u8; 32];
        stored_value.to_big_endian(&mut value_bytes);

        assert_eq!(value_bytes[0..1], [0xAB]);
    }

    #[test]
    fn test_mcopy() {
        let mut vm = VM::default();

        vm.stack.push(U256::from(32)); // size
        vm.stack.push(U256::from(0)); // source offset
        vm.stack.push(U256::from(64)); // destination offset

        vm.stack.push(U256::from(0x33333)); // value
        vm.stack.push(U256::from(0)); // offset

        vm.execute(Bytes::from(vec![
            Opcode::MSTORE as u8,
            Opcode::MCOPY as u8,
            Opcode::STOP as u8,
        ]));

        let copied_value = vm.memory.load(64);
        assert_eq!(copied_value, U256::from(0x33333));
    }

    #[test]
    fn test_mload() {
        let mut vm = VM::default();

        vm.stack.push(U256::from(0)); // offset to load

        vm.stack.push(U256::from(0x33333)); // value to store
        vm.stack.push(U256::from(0)); // offset to store

        vm.execute(Bytes::from(vec![
            Opcode::MSTORE as u8,
            Opcode::MLOAD as u8,
            Opcode::STOP as u8,
        ]));

        let loaded_value = vm.stack.pop().unwrap();
        assert_eq!(loaded_value, U256::from(0x33333));
    }

    #[test]
    fn test_msize() {
        let mut vm = VM::default();

        vm.execute(Bytes::from(vec![Opcode::MSIZE as u8, Opcode::STOP as u8]));
        let initial_size = vm.stack.pop().unwrap();
        assert_eq!(initial_size, U256::from(0));

        vm.pc = 0;

        vm.stack.push(U256::from(0x33333)); // value
        vm.stack.push(U256::from(0)); // offset

        vm.execute(Bytes::from(vec![
            Opcode::MSTORE as u8,
            Opcode::MSIZE as u8,
            Opcode::STOP as u8,
        ]));

        let after_store_size = vm.stack.pop().unwrap();
        assert_eq!(after_store_size, U256::from(32));

        vm.pc = 0;

        vm.stack.push(U256::from(0x55555)); // value
        vm.stack.push(U256::from(64)); // offset

        vm.execute(Bytes::from(vec![
            Opcode::MSTORE as u8,
            Opcode::MSIZE as u8,
            Opcode::STOP as u8,
        ]));

        let final_size = vm.stack.pop().unwrap();
        assert_eq!(final_size, U256::from(96));
    }
}
