use bytes::Bytes;
use ethereum_types::Address;
use secp256k1::{ecdsa, Message, Secp256k1};
use sha3::{Digest, Keccak256};

use crate::constants::{
    ECRECOVER_ADDRESS, ECRECOVER_COST, ECR_HASH_END, ECR_PADDING_LEN, ECR_PARAMS_OFFSET,
    ECR_SIG_END, ECR_V_BASE, ECR_V_POS, IDENTITY_ADDRESS, IDENTITY_STATIC_COST, REVERT_FOR_CALL,
    RIPEMD_160_ADDRESS, RIPEMD_160_STATIC_COST, RIPEMD_OUTPUT_LEN, RIPEMD_PADDING_LEN,
    SHA2_256_ADDRESS, SHA2_256_STATIC_COST, SUCCESS_FOR_CALL,
};

#[derive(Debug, PartialEq)]
pub enum PrecompileError {
    InvalidCalldata,
    NotEnoughGas,
    Secp256k1Error,
    InvalidEcPoint,
}

// Right pads calldata with zeros until specified length
pub fn right_pad(calldata: &Bytes, target_len: usize) -> Bytes {
    let mut padded_calldata = calldata.to_vec();
    if padded_calldata.len() < target_len {
        padded_calldata.extend(vec![0u8; target_len - padded_calldata.len()]);
    }
    padded_calldata.into()
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

pub fn ripemd_160_dynamic_cost(len: u64) -> u64 {
    data_word_size(len) * 120
}

/// ECDSA public key recovery function.
/// More info in https://eips.ethereum.org/EIPS/eip-2, https://eips.ethereum.org/EIPS/eip-1271 and https://www.evm.codes/precompiled.
pub fn ecrecover(
    calldata: &Bytes,
    gas_limit: u64,
    consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    if gas_limit < ECRECOVER_COST {
        return Err(PrecompileError::NotEnoughGas);
    }

    let calldata = right_pad(calldata, ECR_PARAMS_OFFSET);
    let hash = &calldata[..ECR_HASH_END];
    let v = calldata[ECR_V_POS] as i32 - ECR_V_BASE;
    let sig = &calldata[(ECR_V_POS + 1)..ECR_SIG_END];
    let msg = Message::from_digest_slice(hash).map_err(|_| PrecompileError::Secp256k1Error)?;
    let id = ecdsa::RecoveryId::from_i32(v).map_err(|_| PrecompileError::Secp256k1Error)?;
    let sig = ecdsa::RecoverableSignature::from_compact(sig, id)
        .map_err(|_| PrecompileError::Secp256k1Error)?;

    let secp = Secp256k1::new();
    let public_address = secp
        .recover_ecdsa(&msg, &sig)
        .map_err(|_| PrecompileError::Secp256k1Error)?;

    *consumed_gas += ECRECOVER_COST;
    let mut hasher = Keccak256::new();
    hasher.update(&public_address.serialize_uncompressed()[1..]);
    let mut address_hash = hasher.finalize();
    address_hash[..ECR_PADDING_LEN].fill(0);
    Ok(Bytes::copy_from_slice(&address_hash))
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

/// Hashing function.
/// More info in https://github.com/ethereum/yellowpaper.
///
/// # Returns
/// - a 20-byte hash right aligned to 32 bytes
pub fn ripemd_160(
    calldata: &Bytes,
    gas_limit: u64,
    consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    let gas_cost = RIPEMD_160_STATIC_COST + ripemd_160_dynamic_cost(calldata.len() as u64);
    dbg!(gas_cost);
    if gas_limit < gas_cost {
        return Err(PrecompileError::NotEnoughGas);
    }
    *consumed_gas += gas_cost;
    let mut hasher = ripemd::Ripemd160::new();
    hasher.update(calldata);
    let mut output = [0u8; RIPEMD_OUTPUT_LEN];
    hasher.finalize_into((&mut output[RIPEMD_PADDING_LEN..]).into());
    Ok(Bytes::copy_from_slice(&output))
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
) -> (i32, Bytes) {
    let result = match callee_address {
        x if x == Address::from_low_u64_be(ECRECOVER_ADDRESS) => {
            ecrecover(&calldata, gas_to_send, consumed_gas)
        }
        x if x == Address::from_low_u64_be(SHA2_256_ADDRESS) => {
            sha2_256(&calldata, gas_to_send, consumed_gas)
        }
        x if x == Address::from_low_u64_be(RIPEMD_160_ADDRESS) => {
            ripemd_160(&calldata, gas_to_send, consumed_gas)
        }
        x if x == Address::from_low_u64_be(IDENTITY_ADDRESS) => {
            identity(&calldata, gas_to_send, consumed_gas)
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
