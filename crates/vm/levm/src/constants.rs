use crate::errors::VMError;
use ethereum_rust_core::U256;

pub const SUCCESS_FOR_CALL: i32 = 1;
pub const REVERT_FOR_CALL: i32 = 0;
pub const HALT_FOR_CALL: i32 = 2;
pub const SUCCESS_FOR_RETURN: i32 = 1;
pub const REVERT_FOR_CREATE: i32 = 0;
pub const WORD_SIZE: usize = 32;

// Costs in gas for call opcodes (in wei)
pub mod call_opcode {
    use ethereum_rust_core::U256;

    pub const WARM_ADDRESS_ACCESS_COST: U256 = U256([100, 0, 0, 0]);
    pub const COLD_ADDRESS_ACCESS_COST: U256 = U256([2600, 0, 0, 0]);
    pub const NON_ZERO_VALUE_COST: U256 = U256([9000, 0, 0, 0]);
    pub const BASIC_FALLBACK_FUNCTION_STIPEND: U256 = U256([2300, 0, 0, 0]);
    pub const VALUE_TO_EMPTY_ACCOUNT_COST: U256 = U256([25000, 0, 0, 0]);
}
pub const STACK_LIMIT: usize = 1024;

pub const GAS_REFUND_DENOMINATOR: u64 = 5;

pub const EMPTY_CODE_HASH_STR: &str =
    "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470";

pub const MEMORY_EXPANSION_QUOTIENT: usize = 512;

// Transaction costs in gas (in wei)
pub const TX_BASE_COST: U256 = U256([21000, 0, 0, 0]);

pub const MAX_CODE_SIZE: usize = 0x6000;
pub const MAX_CREATE_CODE_SIZE: usize = 2 * MAX_CODE_SIZE;

pub const INVALID_CONTRACT_PREFIX: u8 = 0xef;

// Costs in gas for init word and init code (in wei)
pub const INIT_WORD_COST: usize = 2;

// TODO: See if this function should exist, since has no usages
pub fn init_code_cost(init_code_length: usize) -> Result<u64, VMError> {
    let increased_length = init_code_length
        .checked_add(31)
        .ok_or(VMError::GasCostOverflow)?;
    Ok((INIT_WORD_COST
        .checked_mul(increased_length)
        .ok_or(VMError::GasCostOverflow)?
        / 32) as u64)
}

pub mod create_opcode {
    use ethereum_rust_core::U256;

    pub const INIT_CODE_WORD_COST: U256 = U256([2, 0, 0, 0]);
    pub const CODE_DEPOSIT_COST: U256 = U256([200, 0, 0, 0]);
    pub const CREATE_BASE_COST: U256 = U256([32000, 0, 0, 0]);
}

pub const VERSIONED_HASH_VERSION_KZG: u8 = 0x01;
pub const MAX_BLOB_NUMBER_PER_BLOCK: usize = 6;

// Blob constants
pub const TARGET_BLOB_GAS_PER_BLOCK: U256 = U256([393216, 0, 0, 0]); // TARGET_BLOB_NUMBER_PER_BLOCK * GAS_PER_BLOB
pub const MIN_BASE_FEE_PER_BLOB_GAS: U256 = U256([1, 0, 0, 0]);
pub const BLOB_BASE_FEE_UPDATE_FRACTION: U256 = U256([3338477, 0, 0, 0]);

// Storage constants
pub const COLD_STORAGE_ACCESS_COST: U256 = U256([2100, 0, 0, 0]);

// Block constants
pub const LAST_AVAILABLE_BLOCK_LIMIT: U256 = U256([256, 0, 0, 0]);
pub const MAX_BLOCK_GAS_LIMIT: U256 = U256([30_000_000, 0, 0, 0]);
