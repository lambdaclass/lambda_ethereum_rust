/// Contains the gas costs of the EVM instructions (in wei)
use ethereum_rust_core::U256;

use crate::{
    call_frame::CallFrame,
    constants::{call_opcode::WARM_ADDRESS_ACCESS_COST, WORD_SIZE},
    errors::OutOfGasError,
};

pub const ADD: U256 = U256([3, 0, 0, 0]);
pub const MUL: U256 = U256([5, 0, 0, 0]);
pub const SUB: U256 = U256([3, 0, 0, 0]);
pub const DIV: U256 = U256([5, 0, 0, 0]);
pub const SDIV: U256 = U256([5, 0, 0, 0]);
pub const MOD: U256 = U256([5, 0, 0, 0]);
pub const SMOD: U256 = U256([5, 0, 0, 0]);
pub const ADDMOD: U256 = U256([8, 0, 0, 0]);
pub const MULMOD: U256 = U256([8, 0, 0, 0]);
pub const EXP_STATIC: U256 = U256([10, 0, 0, 0]);
pub const EXP_DYNAMIC_BASE: U256 = U256([50, 0, 0, 0]);
pub const SIGNEXTEND: U256 = U256([5, 0, 0, 0]);
pub const LT: U256 = U256([3, 0, 0, 0]);
pub const GT: U256 = U256([3, 0, 0, 0]);
pub const SLT: U256 = U256([3, 0, 0, 0]);
pub const SGT: U256 = U256([3, 0, 0, 0]);
pub const EQ: U256 = U256([3, 0, 0, 0]);
pub const ISZERO: U256 = U256([3, 0, 0, 0]);
pub const AND: U256 = U256([3, 0, 0, 0]);
pub const OR: U256 = U256([3, 0, 0, 0]);
pub const XOR: U256 = U256([3, 0, 0, 0]);
pub const NOT: U256 = U256([3, 0, 0, 0]);
pub const BYTE: U256 = U256([3, 0, 0, 0]);
pub const SHL: U256 = U256([3, 0, 0, 0]);
pub const SHR: U256 = U256([3, 0, 0, 0]);
pub const SAR: U256 = U256([3, 0, 0, 0]);
pub const KECCAK25_STATIC: U256 = U256([30, 0, 0, 0]);
pub const KECCAK25_DYNAMIC_BASE: U256 = U256([6, 0, 0, 0]);
pub const CALLDATALOAD: U256 = U256([3, 0, 0, 0]);
pub const CALLDATASIZE: U256 = U256([2, 0, 0, 0]);
pub const CALLDATACOPY_STATIC: U256 = U256([3, 0, 0, 0]);
pub const CALLDATACOPY_DYNAMIC_BASE: U256 = U256([3, 0, 0, 0]);
pub const RETURNDATASIZE: U256 = U256([2, 0, 0, 0]);
pub const RETURNDATACOPY_STATIC: U256 = U256([3, 0, 0, 0]);
pub const RETURNDATACOPY_DYNAMIC_BASE: U256 = U256([3, 0, 0, 0]);
pub const ADDRESS: U256 = U256([2, 0, 0, 0]);
pub const ORIGIN: U256 = U256([2, 0, 0, 0]);
pub const CALLER: U256 = U256([2, 0, 0, 0]);
pub const BLOCKHASH: U256 = U256([20, 0, 0, 0]);
pub const COINBASE: U256 = U256([2, 0, 0, 0]);
pub const TIMESTAMP: U256 = U256([2, 0, 0, 0]);
pub const NUMBER: U256 = U256([2, 0, 0, 0]);
pub const PREVRANDAO: U256 = U256([2, 0, 0, 0]);
pub const GASLIMIT: U256 = U256([2, 0, 0, 0]);
pub const CHAINID: U256 = U256([2, 0, 0, 0]);
pub const SELFBALANCE: U256 = U256([5, 0, 0, 0]);
pub const BASEFEE: U256 = U256([2, 0, 0, 0]);
pub const BLOBHASH: U256 = U256([3, 0, 0, 0]);
pub const BLOBBASEFEE: U256 = U256([2, 0, 0, 0]);
pub const POP: U256 = U256([2, 0, 0, 0]);
pub const MLOAD_STATIC: U256 = U256([3, 0, 0, 0]);
pub const MSTORE_STATIC: U256 = U256([3, 0, 0, 0]);
pub const MSTORE8_STATIC: U256 = U256([3, 0, 0, 0]);
pub const JUMP: U256 = U256([8, 0, 0, 0]);
pub const JUMPI: U256 = U256([10, 0, 0, 0]);
pub const PC: U256 = U256([2, 0, 0, 0]);
pub const MSIZE: U256 = U256([2, 0, 0, 0]);
pub const GAS: U256 = U256([2, 0, 0, 0]);
pub const JUMPDEST: U256 = U256([1, 0, 0, 0]);
pub const TLOAD: U256 = U256([100, 0, 0, 0]);
pub const TSTORE: U256 = U256([100, 0, 0, 0]);
pub const MCOPY_STATIC: U256 = U256([3, 0, 0, 0]);
pub const MCOPY_DYNAMIC_BASE: U256 = U256([3, 0, 0, 0]);
pub const PUSH0: U256 = U256([2, 0, 0, 0]);
pub const PUSHN: U256 = U256([3, 0, 0, 0]);
pub const DUPN: U256 = U256([3, 0, 0, 0]);
pub const SWAPN: U256 = U256([3, 0, 0, 0]);
pub const LOGN_STATIC: U256 = U256([375, 0, 0, 0]);
pub const LOGN_DYNAMIC_BASE: U256 = U256([375, 0, 0, 0]);
pub const LOGN_DYNAMIC_BYTE_BASE: U256 = U256([8, 0, 0, 0]);
pub const CALLVALUE: U256 = U256([2, 0, 0, 0]);
pub const CODESIZE: U256 = U256([2, 0, 0, 0]);
pub const CODECOPY_STATIC: U256 = U256([3, 0, 0, 0]);
pub const CODECOPY_DYNAMIC_BASE: U256 = U256([3, 0, 0, 0]);
pub const GASPRICE: U256 = U256([2, 0, 0, 0]);
pub const EXTCODECOPY_DYNAMIC_BASE: U256 = U256([3, 0, 0, 0]);
pub const SELFDESTRUCT_STATIC: U256 = U256([5000, 0, 0, 0]);
pub const SELFDESTRUCT_DYNAMIC: U256 = U256([25000, 0, 0, 0]);
pub const COLD_ADDRESS_ACCESS_COST: U256 = U256([2600, 0, 0, 0]);

pub fn exp_gas_cost(exponent: U256) -> Result<U256, OutOfGasError> {
    let exponent_byte_size = (exponent
        .bits()
        .checked_add(7)
        .ok_or(OutOfGasError::ArithmeticOperationOverflow)? as u64)
        / 8;
    let exponent_byte_size_cost = EXP_DYNAMIC_BASE
        .checked_mul(exponent_byte_size.into())
        .ok_or(OutOfGasError::GasCostOverflow)?;
    EXP_STATIC
        .checked_add(exponent_byte_size_cost)
        .ok_or(OutOfGasError::GasCostOverflow)
}

pub fn calldatacopy_gas_cost(
    current_call_frame: &mut CallFrame,
    size: usize,
    dest_offset: usize,
) -> Result<U256, OutOfGasError> {
    let minimum_word_size = (size
        .checked_add(WORD_SIZE)
        .ok_or(OutOfGasError::ArithmeticOperationOverflow)?
        .saturating_sub(1))
        / WORD_SIZE;

    let memory_expansion_cost = current_call_frame.memory.expansion_cost(
        dest_offset
            .checked_add(size)
            .ok_or(OutOfGasError::ArithmeticOperationOverflow)?,
    )?;

    let minimum_word_size_cost = CALLDATACOPY_DYNAMIC_BASE
        .checked_mul(minimum_word_size.into())
        .ok_or(OutOfGasError::GasCostOverflow)?;
    CALLDATACOPY_STATIC
        .checked_add(minimum_word_size_cost)
        .ok_or(OutOfGasError::GasCostOverflow)?
        .checked_add(memory_expansion_cost)
        .ok_or(OutOfGasError::GasCostOverflow)
}

pub fn codecopy_gas_cost(
    current_call_frame: &mut CallFrame,
    size: usize,
    dest_offset: usize,
) -> Result<U256, OutOfGasError> {
    let minimum_word_size = (size
        .checked_add(WORD_SIZE)
        .ok_or(OutOfGasError::ArithmeticOperationOverflow)?
        .saturating_sub(1))
        / WORD_SIZE;

    let memory_expansion_cost = current_call_frame.memory.expansion_cost(
        dest_offset
            .checked_add(size)
            .ok_or(OutOfGasError::ArithmeticOperationOverflow)?,
    )?;

    let minimum_word_size_cost = CODECOPY_DYNAMIC_BASE
        .checked_mul(minimum_word_size.into())
        .ok_or(OutOfGasError::GasCostOverflow)?;
    CODECOPY_STATIC
        .checked_add(minimum_word_size_cost)
        .ok_or(OutOfGasError::GasCostOverflow)?
        .checked_add(memory_expansion_cost)
        .ok_or(OutOfGasError::GasCostOverflow)
}

pub fn extcodecopy_gas_cost(
    current_call_frame: &mut CallFrame,
    size: usize,
    dest_offset: usize,
    is_cached: bool,
) -> Result<U256, OutOfGasError> {
    let minimum_word_size = (size
        .checked_add(WORD_SIZE)
        .ok_or(OutOfGasError::ArithmeticOperationOverflow)?
        .saturating_sub(1))
        / WORD_SIZE;

    let memory_expansion_cost = current_call_frame.memory.expansion_cost(
        dest_offset
            .checked_add(size)
            .ok_or(OutOfGasError::ArithmeticOperationOverflow)?,
    )?;
    let minimum_word_size_cost = EXTCODECOPY_DYNAMIC_BASE
        .checked_add(minimum_word_size.into())
        .ok_or(OutOfGasError::GasCostOverflow)?;

    let address_access_cost = if is_cached {
        WARM_ADDRESS_ACCESS_COST
    } else {
        COLD_ADDRESS_ACCESS_COST
    };

    minimum_word_size_cost
        .checked_add(memory_expansion_cost)
        .ok_or(OutOfGasError::GasCostOverflow)?
        .checked_add(address_access_cost)
        .ok_or(OutOfGasError::GasCostOverflow)
}

pub fn returndatacopy_gas_cost(current_call_frame: &mut CallFrame, size: usize, dest_offset: usize) -> Result<U256, OutOfGasError> {
    let minimum_word_size = (size
        .checked_add(WORD_SIZE)
        .ok_or(
            OutOfGasError::ArithmeticOperationOverflow,
        )?
        .saturating_sub(1))
        / WORD_SIZE;
    let memory_expansion_cost =
        current_call_frame
            .memory
            .expansion_cost(dest_offset.checked_add(size).ok_or(
                OutOfGasError::ArithmeticOperationOverflow,
            )?)?;
    let minumum_word_size_cost = RETURNDATACOPY_DYNAMIC_BASE
        .checked_mul(minimum_word_size.into())
        .ok_or(OutOfGasError::GasCostOverflow)?;

    RETURNDATACOPY_STATIC
        .checked_add(minumum_word_size_cost)
        .ok_or(OutOfGasError::GasCostOverflow)?
        .checked_add(memory_expansion_cost)
        .ok_or(OutOfGasError::GasCostOverflow)
}

pub fn keccak256_gas_cost(
    current_call_frame: &mut CallFrame,
    size: usize,
    offset: usize,
) -> Result<U256, OutOfGasError> {
    let minimum_word_size = (size
        .checked_add(WORD_SIZE)
        .ok_or(OutOfGasError::ArithmeticOperationOverflow)?
        .saturating_sub(1))
        / WORD_SIZE;
    let memory_expansion_cost = current_call_frame.memory.expansion_cost(
        offset
            .checked_add(size)
            .ok_or(OutOfGasError::ArithmeticOperationOverflow)?,
    )?;
    let minimum_word_size_cost = KECCAK25_DYNAMIC_BASE
        .checked_mul(minimum_word_size.into())
        .ok_or(OutOfGasError::GasCostOverflow)?;

    KECCAK25_STATIC
        .checked_add(minimum_word_size_cost)
        .ok_or(OutOfGasError::GasCostOverflow)?
        .checked_add(memory_expansion_cost)
        .ok_or(OutOfGasError::GasCostOverflow)
}

