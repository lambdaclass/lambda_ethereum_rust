use crate::constants::{MEMORY_EXPANSION_QUOTIENT, WORD_SIZE};
use crate::primitives::U256;
use crate::vm_result::VMError;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Memory {
    data: Vec<u8>,
}

impl From<Vec<u8>> for Memory {
    fn from(data: Vec<u8>) -> Self {
        Memory { data }
    }
}

impl Memory {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn new_from_vec(data: Vec<u8>) -> Self {
        Self { data }
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

    pub fn load_range(&mut self, offset: usize, size: usize) -> Vec<u8> {
        self.resize(offset + size);
        self.data.get(offset..offset + size).unwrap().into()
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

    pub fn expansion_cost(&self, memory_byte_size: usize) -> Result<usize, VMError> {
        if memory_byte_size <= self.data.len() {
            return Ok(0);
        }
        let new_memory_size_word = memory_byte_size
            .checked_add(WORD_SIZE - 1)
            .ok_or(VMError::OverflowInArithmeticOp)?
            / WORD_SIZE;
        // (new_memory_size_word * new_memory_size_word) / 512 + (3 * new_memory_size_word);
        let new_memory_cost = new_memory_size_word
            .checked_mul(new_memory_size_word)
            .and_then(|square| square.checked_div(MEMORY_EXPANSION_QUOTIENT))
            .and_then(|cost| cost.checked_add(new_memory_size_word.checked_mul(3)?))
            .ok_or(VMError::OverflowInArithmeticOp)?;
        let last_memory_size_word = (self.data.len() + WORD_SIZE - 1) / WORD_SIZE;
        let last_memory_cost =
            (last_memory_size_word * last_memory_size_word) / MEMORY_EXPANSION_QUOTIENT + (3 * last_memory_size_word);
        Ok(new_memory_cost - last_memory_cost)
    }
}
