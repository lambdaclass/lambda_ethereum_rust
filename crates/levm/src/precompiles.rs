use bytes::Bytes;
use ethereum_types::Address;

use crate::constants::{REVERT_FOR_CALL, SUCCESS_FOR_CALL};

#[derive(Debug, PartialEq)]
pub enum PrecompileError {
    InvalidCalldata,
    NotEnoughGas,
    Secp256k1Error,
    InvalidEcPoint,
}

pub const IDENTITY_STATIC_COST: u64 = 15;
pub const IDENTITY_ADDRESS: u64 = 0x04;

pub fn identity_dynamic_cost(len: u64) -> u64 {
    let data_word_size = (len + 31) / 32;
    data_word_size * 3
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

pub fn execute_precompile(
    callee_address: Address,
    calldata: Bytes,
    gas_to_send: u64,
    consumed_gas: &mut u64,
) -> (u8, Bytes) {
    let result = match callee_address {
        x if x == Address::from_low_u64_be(IDENTITY_ADDRESS) => {
            identity(&calldata, gas_to_send, consumed_gas)
        }
        _ => {
            unreachable!()
        }
    };
    match result {
        Ok(res) => (SUCCESS_FOR_CALL as u8, res),
        Err(_) => {
            *consumed_gas += gas_to_send;
            (REVERT_FOR_CALL as u8, Bytes::new())
        }
    }
}
