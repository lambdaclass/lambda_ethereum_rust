use crate::{
    constants::{MEMORY_EXPANSION_QUOTIENT, WORD_SIZE},
    errors::{InternalError, OutOfGasError, VMError},
};
use ethereum_rust_core::U256;

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
        if offset.next_multiple_of(WORD_SIZE) > self.data.len() {
            self.data.resize(offset.next_multiple_of(WORD_SIZE), 0);
        }
    }

    pub fn load(&mut self, offset: usize) -> Result<U256, VMError> {
        self.resize(offset.checked_add(WORD_SIZE).ok_or(VMError::Internal(
            InternalError::ArithmeticOperationOverflow,
        ))?);
        let value_bytes: [u8; WORD_SIZE] = self
            .data
            .get(
                offset
                    ..offset.checked_add(WORD_SIZE).ok_or(VMError::Internal(
                        InternalError::ArithmeticOperationOverflow,
                    ))?,
            )
            .ok_or(VMError::MemoryLoadOutOfBounds)?
            .try_into()
            .unwrap();
        Ok(U256::from(value_bytes))
    }

    pub fn load_range(&mut self, offset: usize, size: usize) -> Result<Vec<u8>, VMError> {
        let size_to_load = offset.checked_add(size).ok_or(VMError::Internal(
            InternalError::ArithmeticOperationOverflow,
        ))?;
        self.resize(size_to_load);
        self.data
            .get(offset..size_to_load)
            .ok_or(VMError::MemoryLoadOutOfBounds)
            .map(|slice| slice.to_vec())
    }

    pub fn store_bytes(&mut self, offset: usize, value: &[u8]) -> Result<(), VMError> {
        let len = value.len();
        let data_len = self.data.len();
        let size_to_store = offset.checked_add(len).ok_or(VMError::Internal(
            InternalError::ArithmeticOperationOverflow,
        ))?;
        if data_len < offset || data_len < size_to_store {
            return Err(VMError::MemoryStoreOutOfBounds);
        }
        self.resize(size_to_store);
        self.data
            .splice(offset..size_to_store, value.iter().copied());

        Ok(())
    }

    pub fn store_n_bytes(
        &mut self,
        offset: usize,
        value: &[u8],
        size: usize,
    ) -> Result<(), VMError> {
        let size_to_store = offset.checked_add(size).ok_or(VMError::Internal(
            InternalError::ArithmeticOperationOverflow,
        ))?;
        self.resize(size_to_store);
        self.data
            .splice(offset..size_to_store, value.iter().copied());

        Ok(())
    }

    pub fn size(&self) -> U256 {
        U256::from(self.data.len())
    }

    pub fn copy(
        &mut self,
        src_offset: usize,
        dest_offset: usize,
        size: usize,
    ) -> Result<(), VMError> {
        let src_copy_size = src_offset.checked_add(size).ok_or(VMError::Internal(
            InternalError::ArithmeticOperationOverflow,
        ))?;
        let dest_copy_size = dest_offset.checked_add(size).ok_or(VMError::Internal(
            InternalError::ArithmeticOperationOverflow,
        ))?;
        let max_size = std::cmp::max(src_copy_size, dest_copy_size);
        self.resize(max_size);
        let mut temp = vec![0u8; size];

        temp.copy_from_slice(&self.data[src_offset..src_copy_size]);

        self.data[dest_offset..dest_copy_size].copy_from_slice(&temp);

        Ok(())
    }

    pub fn expansion_cost(&self, memory_byte_size: usize) -> Result<U256, OutOfGasError> {
        if memory_byte_size <= self.data.len() {
            return Ok(U256::zero());
        }

        let new_memory_size_word = memory_byte_size
            .checked_add(WORD_SIZE - 1)
            .ok_or(OutOfGasError::ArithmeticOperationOverflow)?
            / WORD_SIZE;

        let new_memory_cost = new_memory_size_word
            .checked_mul(new_memory_size_word)
            .map(|square| square / MEMORY_EXPANSION_QUOTIENT)
            .and_then(|cost| cost.checked_add(new_memory_size_word.checked_mul(3)?))
            .ok_or(OutOfGasError::ArithmeticOperationOverflow)?;

        let last_memory_size_word = self
            .data
            .len()
            .checked_add(WORD_SIZE - 1)
            .ok_or(OutOfGasError::ArithmeticOperationOverflow)?
            / WORD_SIZE;

        let last_memory_cost = last_memory_size_word
            .checked_mul(last_memory_size_word)
            .map(|square| square / MEMORY_EXPANSION_QUOTIENT)
            .and_then(|cost| cost.checked_add(last_memory_size_word.checked_mul(3)?))
            .ok_or(OutOfGasError::ArithmeticOperationOverflow)?;

        Ok((new_memory_cost
            .checked_sub(last_memory_cost)
            .ok_or(OutOfGasError::GasCostOverflow)?)
        .into())
    }
}
