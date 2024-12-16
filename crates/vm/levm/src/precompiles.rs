use bytes::Bytes;
use ethrex_core::{Address, H160, U256};
use keccak_hash::keccak256;
use libsecp256k1::{self, Message, RecoveryId, Signature};

use crate::{
    call_frame::CallFrame,
    constants::{REVERT_FOR_RETURN, SUCCESS_FOR_RETURN},
    errors::{InternalError, PrecompileError, VMError},
    gas_cost::ECRECOVER_COST,
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

// Check if it is right
pub const SECP256K1N: U256 = U256([
    0xBAAEDCE6AF48A03B,
    0xBFD25E8CD0364141,
    0xFFFFFFFFFFFFFFFF,
    0xFFFFFFFFFFFFFFFF,
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

pub fn is_precompile(callee_address: &Address) -> bool {
    PRECOMPILES.contains(callee_address)
}

pub fn execute_precompile(current_call_frame: &mut CallFrame) -> Result<(u8, Bytes), VMError> {
    let callee_address = current_call_frame.code_address;
    let calldata = current_call_frame.calldata.clone();
    let gas_for_call = current_call_frame.gas_limit;
    let consumed_gas = &mut current_call_frame.gas_used;

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
        _ => return Err(VMError::Internal(InternalError::InvalidPrecompileAddress)),
    };
    match result {
        Ok(res) => Ok((SUCCESS_FOR_RETURN, res)),
        Err(_) => {
            // Maybe we should return an Err in this case. Like differencing between OOG,
            // errors produced by wrong inputs an internal errors
            *consumed_gas = consumed_gas
                .checked_add(gas_for_call)
                .ok_or(InternalError::ArithmeticOperationOverflow)?;
            Ok((REVERT_FOR_RETURN, Bytes::new()))
        }
    }
}

fn ecrecover(
    calldata: &Bytes,
    _gas_for_call: U256,
    consumed_gas: &mut U256,
) -> Result<Bytes, PrecompileError> {
    // If calldata does not reach the required length, we should fill the rest with zeros

    let hash = calldata
        .get(0..32)
        .ok_or(PrecompileError::ParsingInputError)?;
    let message = Message::parse_slice(hash).map_err(|_| PrecompileError::ParsingInputError)?;

    let r: &u8 = calldata.get(63).ok_or(PrecompileError::ParsingInputError)?;
    let recovery_id = RecoveryId::parse_rpc(*r).map_err(|_| PrecompileError::ParsingInputError)?;

    let sig = calldata
        .get(64..128)
        .ok_or(PrecompileError::ParsingInputError)?;
    let signature =
        Signature::parse_standard_slice(sig).map_err(|_| PrecompileError::ParsingInputError)?;

    // Consume gas
    *consumed_gas = consumed_gas
        .checked_add(ECRECOVER_COST.into())
        .ok_or(PrecompileError::GasConsumedOverflow)?;

    let mut public_key = libsecp256k1::recover(&message, &signature, &recovery_id)
        .map_err(|_| PrecompileError::KeyRecoverError)?
        .serialize();

    keccak256(&mut public_key[1..65]);

    let mut result = [0u8; 32];
    // To-do: use a non panicking way to copy the bytes
    result[12..32].copy_from_slice(&public_key);

    Ok(Bytes::from(result.to_vec()))
}

fn identity(
    _calldata: &Bytes,
    _gas_for_call: U256,
    _consumed_gas: &mut U256,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn sha2_256(
    _calldata: &Bytes,
    _gas_for_call: U256,
    _consumed_gas: &mut U256,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn ripemd_160(
    _calldata: &Bytes,
    _gas_for_call: U256,
    _consumed_gas: &mut U256,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn modexp(
    _calldata: &Bytes,
    _gas_for_call: U256,
    _consumed_gas: &mut U256,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn ecadd(
    _calldata: &Bytes,
    _gas_for_call: U256,
    _consumed_gas: &mut U256,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn ecmul(
    _calldata: &Bytes,
    _gas_for_call: U256,
    _consumed_gas: &mut U256,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn ecpairing(
    _calldata: &Bytes,
    _gas_for_call: U256,
    _consumed_gas: &mut U256,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn blake2f(
    _calldata: &Bytes,
    _gas_for_call: U256,
    _consumed_gas: &mut U256,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}

fn point_evaluation(
    _calldata: &Bytes,
    _gas_for_call: U256,
    _consumed_gas: &mut U256,
) -> Result<Bytes, PrecompileError> {
    Ok(Bytes::new())
}
