use bytes::Bytes;
use ethrex_core::{Address, H160, U256};
use keccak_hash::keccak256;
use libsecp256k1::{self, Message, RecoveryId, Signature};
use sha3::Digest;

use crate::{
    call_frame::CallFrame,
    errors::{InternalError, PrecompileError, VMError},
    gas_cost::{
        identity as identity_cost, modexp as modexp_cost, ripemd_160 as ripemd_160_cost,
        sha2_256 as sha2_256_cost, ECRECOVER_COST,
    },
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

pub fn is_precompile(callee_address: &Address) -> bool {
    PRECOMPILES.contains(callee_address)
}

pub fn execute_precompile(current_call_frame: &mut CallFrame) -> Result<Bytes, VMError> {
    let callee_address = current_call_frame.code_address;
    let calldata = current_call_frame.calldata.clone();
    let gas_for_call = current_call_frame.gas_limit;
    let consumed_gas = &mut current_call_frame.gas_used;

    let result = match callee_address {
        address if address == ECRECOVER_ADDRESS => {
            ecrecover(&calldata, gas_for_call, consumed_gas)?
        }
        address if address == IDENTITY_ADDRESS => identity(&calldata, gas_for_call, consumed_gas)?,
        address if address == SHA2_256_ADDRESS => sha2_256(&calldata, gas_for_call, consumed_gas)?,
        address if address == RIPEMD_160_ADDRESS => {
            ripemd_160(&calldata, gas_for_call, consumed_gas)?
        }
        address if address == MODEXP_ADDRESS => modexp(&calldata, gas_for_call, consumed_gas)?,
        address if address == ECADD_ADDRESS => ecadd(&calldata, gas_for_call, consumed_gas)?,
        address if address == ECMUL_ADDRESS => ecmul(&calldata, gas_for_call, consumed_gas)?,
        address if address == ECPAIRING_ADDRESS => {
            ecpairing(&calldata, gas_for_call, consumed_gas)?
        }
        address if address == BLAKE2F_ADDRESS => blake2f(&calldata, gas_for_call, consumed_gas)?,
        address if address == POINT_EVALUATION_ADDRESS => {
            point_evaluation(&calldata, gas_for_call, consumed_gas)?
        }
        _ => return Err(VMError::Internal(InternalError::InvalidPrecompileAddress)),
    };

    Ok(result)
}

fn fill_with_zeros(slice: &[u8]) -> [u8; 128] {
    let mut result = [0; 128];

    let n = slice.len().min(128);

    let trimmed_slice = slice.get(..n).unwrap_or_default();
    result
        .get_mut(..n)
        .unwrap_or_default()
        .copy_from_slice(trimmed_slice);

    result
}

pub fn ecrecover(
    calldata: &Bytes,
    gas_for_call: U256,
    consumed_gas: &mut U256,
) -> Result<Bytes, VMError> {
    // Consume gas
    *consumed_gas = consumed_gas
        .checked_add(ECRECOVER_COST.into())
        .ok_or(PrecompileError::GasConsumedOverflow)?;

    if gas_for_call < *consumed_gas {
        return Err(VMError::PrecompileError(PrecompileError::NotEnoughGas));
    }

    // If calldata does not reach the required length, we should fill the rest with zeros
    let calldata = fill_with_zeros(calldata);

    let hash = calldata
        .get(0..32)
        .ok_or(PrecompileError::ParsingInputError)?;
    let message = Message::parse_slice(hash).map_err(|_| PrecompileError::ParsingInputError)?;

    let r = calldata.get(63).ok_or(PrecompileError::ParsingInputError)?;
    let recovery_id = match RecoveryId::parse_rpc(*r) {
        Ok(id) => id,
        Err(_) => {
            return Ok(Bytes::new());
        }
    };

    let sig = calldata
        .get(64..128)
        .ok_or(PrecompileError::ParsingInputError)?;
    let signature =
        Signature::parse_standard_slice(sig).map_err(|_| PrecompileError::ParsingInputError)?;

    let mut public_key = match libsecp256k1::recover(&message, &signature, &recovery_id) {
        Ok(id) => id,
        Err(_) => {
            return Ok(Bytes::new());
        }
    }
    .serialize();

    keccak256(&mut public_key[1..65]);

    let mut output = vec![0u8; 12];
    output.extend_from_slice(&public_key[13..33]);

    Ok(Bytes::from(output.to_vec()))
}

fn identity(
    calldata: &Bytes,
    gas_for_call: U256,
    consumed_gas: &mut U256,
) -> Result<Bytes, VMError> {
    let data_size: u64 = calldata
        .len()
        .try_into()
        .map_err(|_| PrecompileError::ParsingInputError)?;

    let cost = identity_cost(data_size)?;
    if gas_for_call < cost {
        return Err(VMError::PrecompileError(PrecompileError::NotEnoughGas));
    }

    *consumed_gas = consumed_gas
        .checked_add(cost)
        .ok_or(PrecompileError::GasConsumedOverflow)?;

    Ok(calldata.clone())
}

fn sha2_256(
    calldata: &Bytes,
    gas_for_call: U256,
    consumed_gas: &mut U256,
) -> Result<Bytes, VMError> {
    let data_size: u64 = calldata
        .len()
        .try_into()
        .map_err(|_| PrecompileError::ParsingInputError)?;

    let cost = sha2_256_cost(data_size)?;
    if gas_for_call < cost {
        return Err(VMError::PrecompileError(PrecompileError::NotEnoughGas));
    }

    *consumed_gas = consumed_gas
        .checked_add(cost)
        .ok_or(PrecompileError::GasConsumedOverflow)?;

    let result = sha2::Sha256::digest(calldata).to_vec();

    Ok(Bytes::from(result))
}

fn ripemd_160(
    calldata: &Bytes,
    gas_for_call: U256,
    consumed_gas: &mut U256,
) -> Result<Bytes, VMError> {
    let data_size: u64 = calldata
        .len()
        .try_into()
        .map_err(|_| PrecompileError::ParsingInputError)?;

    let cost = ripemd_160_cost(data_size)?;
    if gas_for_call < cost {
        return Err(VMError::PrecompileError(PrecompileError::NotEnoughGas));
    }

    *consumed_gas = consumed_gas
        .checked_add(cost)
        .ok_or(PrecompileError::GasConsumedOverflow)?;

    let mut hasher = ripemd::Ripemd160::new();
    hasher.update(calldata);
    let result = hasher.finalize();

    let mut output = vec![0; 12];
    output.extend_from_slice(&result);

    Ok(Bytes::from(output.to_vec()))
}

pub fn modexp(
    calldata: &Bytes,
    gas_for_call: U256,
    consumed_gas: &mut U256,
) -> Result<Bytes, VMError> {
    // Note that fill_with_zeros defines a fixed size slice, not optimal
    let calldata = fill_with_zeros(calldata);

    let b_size: U256 = calldata
        .get(0..32)
        .ok_or(PrecompileError::ParsingInputError)?
        .into();
    let b_size = usize::try_from(b_size).map_err(|_| PrecompileError::ParsingInputError)?;

    let e_size: U256 = calldata
        .get(32..64)
        .ok_or(PrecompileError::ParsingInputError)?
        .into();
    let e_size = usize::try_from(e_size).map_err(|_| PrecompileError::ParsingInputError)?;

    let m_size: U256 = calldata
        .get(64..96)
        .ok_or(PrecompileError::ParsingInputError)?
        .into();
    let m_size = usize::try_from(m_size).map_err(|_| PrecompileError::ParsingInputError)?;

    let base_limit = b_size
        .checked_add(96)
        .ok_or(InternalError::ArithmeticOperationOverflow)?;

    let base: U256 = calldata
        .get(96..base_limit)
        .ok_or(PrecompileError::ParsingInputError)?
        .into();

    let exponent_limit = e_size
        .checked_add(base_limit)
        .ok_or(InternalError::ArithmeticOperationOverflow)?;

    let exponent: U256 = calldata
        .get(base_limit..exponent_limit)
        .ok_or(PrecompileError::ParsingInputError)?
        .into();

    let modulus_limit = m_size
        .checked_add(exponent_limit)
        .ok_or(InternalError::ArithmeticOperationOverflow)?;

    let modulus: U256 = calldata
        .get(exponent_limit..modulus_limit)
        .ok_or(PrecompileError::ParsingInputError)?
        .into();

    let gas_cost = modexp_cost(exponent, b_size, e_size, m_size)?;

    if gas_for_call < gas_cost {
        return Err(VMError::PrecompileError(PrecompileError::NotEnoughGas));
    }

    *consumed_gas = consumed_gas
        .checked_add(gas_cost)
        .ok_or(PrecompileError::GasConsumedOverflow)?;

    let result = mod_exp(base, exponent, modulus)?;

    let res_bytes = result.as_usize().to_be_bytes().to_vec();
    let res_bytes = increase_left_pad(&Bytes::from(res_bytes), m_size)?;

    let size_diff = (res_bytes.len())
        .checked_sub(m_size)
        .ok_or(InternalError::ArithmeticOperationUnderflow)?;
    Ok(res_bytes.slice(size_diff..))
}

pub fn increase_left_pad(result: &Bytes, m_size: usize) -> Result<Bytes, VMError> {
    let mut padded_result = vec![0u8; m_size];
    if result.len() < m_size {
        let size_diff = m_size
            .checked_sub(result.len())
            .ok_or(InternalError::ArithmeticOperationUnderflow)?;
        padded_result
            .get_mut(size_diff..)
            .ok_or(InternalError::SlicingError)?
            .copy_from_slice(result);

        Ok(padded_result.into())
    } else {
        Ok(result.clone())
    }
}

fn mod_exp(b: U256, e: U256, m: U256) -> Result<U256, PrecompileError> {
    let mut result = U256::one();
    let mut base = b.checked_rem(m).ok_or(PrecompileError::DefaultError)?;
    let mut exponent = e;

    while exponent > U256::zero() {
        if exponent
            .checked_rem(2.into())
            .ok_or(PrecompileError::DefaultError)?
            == U256::one()
        {
            result = (result
                .checked_mul(base)
                .ok_or(PrecompileError::DefaultError)?)
            .checked_rem(m)
            .ok_or(PrecompileError::DefaultError)?;
        }
        base = (base
            .checked_mul(base)
            .ok_or(PrecompileError::DefaultError)?)
        .checked_rem(m)
        .ok_or(PrecompileError::DefaultError)?;
        exponent = exponent
            .checked_div(U256::from(2))
            .ok_or(PrecompileError::DefaultError)?;
    }

    Ok(result)
}

fn ecadd(
    _calldata: &Bytes,
    _gas_for_call: U256,
    _consumed_gas: &mut U256,
) -> Result<Bytes, VMError> {
    Ok(Bytes::new())
}

fn ecmul(
    _calldata: &Bytes,
    _gas_for_call: U256,
    _consumed_gas: &mut U256,
) -> Result<Bytes, VMError> {
    Ok(Bytes::new())
}

fn ecpairing(
    _calldata: &Bytes,
    _gas_for_call: U256,
    _consumed_gas: &mut U256,
) -> Result<Bytes, VMError> {
    Ok(Bytes::new())
}

fn blake2f(
    _calldata: &Bytes,
    _gas_for_call: U256,
    _consumed_gas: &mut U256,
) -> Result<Bytes, VMError> {
    Ok(Bytes::new())
}

fn point_evaluation(
    _calldata: &Bytes,
    _gas_for_call: U256,
    _consumed_gas: &mut U256,
) -> Result<Bytes, VMError> {
    Ok(Bytes::new())
}
