use bytes::Bytes;
use ethereum_types::Address;
use sha3::Digest;

use crate::constants::{
    IDENTITY_ADDRESS, IDENTITY_STATIC_COST, REVERT_FOR_CALL, SHA2_256_ADDRESS,
    SHA2_256_STATIC_COST, SUCCESS_FOR_CALL,
};

#[derive(Debug, PartialEq)]
pub enum PrecompileError {
    InvalidCalldata,
    NotEnoughGas,
    Secp256k1Error,
    InvalidEcPoint,
}

pub fn data_word_size(len: u64) -> u64 {
    (len + 31) / 32
}

pub fn identity_dynamic_cost(len: u64) -> u64 {
    data_word_size(len) * 3
}

pub fn sha2_256_dynamic_cost(len: u64) -> u64 {
    data_word_size(len) * 12
}

/// The identity function is typically used to copy a chunk of memory. It copies its input to its output. It can be used to copy between memory portions.
/// More info in https://github.com/ethereum/yellowpaper.
pub fn identity(
    calldata: &Bytes,
    gas_limit: u64,
    consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    let gas_cost = IDENTITY_STATIC_COST + identity_dynamic_cost(calldata.len() as u64);
    if gas_limit < gas_cost {
        return Err(PrecompileError::NotEnoughGas);
    }
    *consumed_gas += gas_cost;
    Ok(calldata.clone())
}

/// Hashing function.
/// More info in https://github.com/ethereum/yellowpaper.
pub fn sha2_256(
    calldata: &Bytes,
    gas_limit: u64,
    consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    let gas_cost = SHA2_256_STATIC_COST + sha2_256_dynamic_cost(calldata.len() as u64);
    if gas_limit < gas_cost {
        return Err(PrecompileError::NotEnoughGas);
    }
    *consumed_gas += gas_cost;
    let hash = sha2::Sha256::digest(calldata);
    Ok(Bytes::copy_from_slice(&hash))
}

pub fn execute_precompile(
    callee_address: Address,
    calldata: Bytes,
    gas_to_send: u64,
    consumed_gas: &mut u64,
) -> (i32, Bytes) {
    let result = match callee_address {
        x if x == Address::from_low_u64_be(IDENTITY_ADDRESS) => {
            identity(&calldata, gas_to_send, consumed_gas)
        }
        x if x == Address::from_low_u64_be(SHA2_256_ADDRESS) => {
            sha2_256(&calldata, gas_to_send, consumed_gas)
        }
        _ => {
            unreachable!()
        }
    };
    match result {
        Ok(res) => (SUCCESS_FOR_CALL, res),
        Err(_) => {
            *consumed_gas += gas_to_send;
            (REVERT_FOR_CALL, Bytes::new())
        }
    }
}
