use bytes::Bytes;
use ethrex_core::{types::TxKind, Address, H160};

use crate::{
    constants::{REVERT_FOR_RETURN, SUCCESS_FOR_RETURN},
    errors::{InternalError, PrecompileError},
};

pub const ECRECOVER_ADDRESS: H160 = H160([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x01,
]);
pub const SHA2_256_ADDRESS: H160 = H160([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x02,
]);
pub const RIPEMD_160_ADDRESS: H160 = H160([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x03,
]);
pub const IDENTITY_ADDRESS: H160 = H160([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x04,
]);
pub const MODEXP_ADDRESS: H160 = H160([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x05,
]);
pub const ECADD_ADDRESS: H160 = H160([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x06,
]);
pub const ECMUL_ADDRESS: H160 = H160([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x07,
]);
pub const ECPAIRING_ADDRESS: H160 = H160([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x08,
]);
pub const BLAKE2F_ADDRESS: H160 = H160([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x09,
]);
pub const POINT_EVALUATION_ADDRESS: H160 = H160([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x0a,
]);

pub const PRECOMPILES: [H160; 10] = [
    ECRECOVER_ADDRESS,
    SHA2_256_ADDRESS,
    RIPEMD_160_ADDRESS,
    IDENTITY_ADDRESS,
    MODEXP_ADDRESS,
    ECADD_ADDRESS,
    ECMUL_ADDRESS,
    ECPAIRING_ADDRESS,
    BLAKE2F_ADDRESS,
    POINT_EVALUATION_ADDRESS,
];

pub fn is_precompile(callee_address: Address) -> bool {
    PRECOMPILES.contains(&callee_address)
}

pub fn execute_precompile(
    callee_address: Address,
    calldata: Bytes,
    gas_for_call: u64,
    consumed_gas: &mut u64,
) -> Result<(u8, Bytes), InternalError> {
    let result = match callee_address {
        address if address == ECRECOVER_ADDRESS => ecrecover(&calldata, gas_for_call, consumed_gas),
        address if address == IDENTITY_ADDRESS => identity(&calldata, gas_for_call, consumed_gas),
        address if address == SHA2_256_ADDRESS => sha2_256(&calldata, gas_for_call, consumed_gas),
        address if address == RIPEMD_160_ADDRESS => {
            ripemd_160(&calldata, gas_for_call, consumed_gas)
        }
        address if address == MODEXP_ADDRESS => modexp(&calldata, gas_for_call, consumed_gas),
        address if address == ECADD_ADDRESS => ecadd(&calldata, gas_for_call, consumed_gas),
        address if address == ECMUL_ADDRESS => ecmul(&calldata, gas_for_call, consumed_gas),
        address if address == ECPAIRING_ADDRESS => ecpairing(&calldata, gas_for_call, consumed_gas),
        address if address == BLAKE2F_ADDRESS => blake2f(&calldata, gas_for_call, consumed_gas),
        address if address == POINT_EVALUATION_ADDRESS => {
            point_evaluation(&calldata, gas_for_call, consumed_gas)
        }
        _ => return Err(InternalError::InvalidPrecompileAddress),
    };
    match result {
        Ok(res) => Ok((SUCCESS_FOR_RETURN, res)),
        Err(_) => {
            *consumed_gas = consumed_gas
                .checked_add(gas_for_call)
                .ok_or(InternalError::ArithmeticOperationOverflow)?;
            Ok((REVERT_FOR_RETURN, Bytes::new()))
        }
    }
}

fn ecrecover(
    _calldata: &Bytes,
    _gas_for_call: u64,
    _consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn identity(
    _calldata: &Bytes,
    _gas_for_call: u64,
    _consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn sha2_256(
    _calldata: &Bytes,
    _gas_for_call: u64,
    _consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn ripemd_160(
    _calldata: &Bytes,
    _gas_for_call: u64,
    _consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn modexp(
    _calldata: &Bytes,
    _gas_for_call: u64,
    _consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn ecadd(
    _calldata: &Bytes,
    _gas_for_call: u64,
    _consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn ecmul(
    _calldata: &Bytes,
    _gas_for_call: u64,
    _consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn ecpairing(
    _calldata: &Bytes,
    _gas_for_call: u64,
    _consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn blake2f(
    _calldata: &Bytes,
    _gas_for_call: u64,
    _consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn point_evaluation(
    _calldata: &Bytes,
    _gas_for_call: u64,
    _consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}
