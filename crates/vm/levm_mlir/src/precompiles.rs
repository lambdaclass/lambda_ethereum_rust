use crate::{
    constants::{
        precompiles::*,
        return_codes::{REVERT_RETURN_CODE, SUCCESS_RETURN_CODE},
    },
    primitives::U256,
    result::PrecompileError,
    utils::{left_pad, right_pad},
};
use bytes::Bytes;
use ethereum_types::Address;
use lambdaworks_math::{
    cyclic_group::IsGroup,
    elliptic_curve::{
        short_weierstrass::curves::bn_254::{
            curve::{BN254Curve, BN254FieldElement, BN254TwistCurveFieldElement},
            field_extension::Degree12ExtensionField,
            pairing::BN254AtePairing,
            twist::BN254TwistCurve,
        },
        traits::{IsEllipticCurve, IsPairing},
    },
    field::{element::FieldElement, extensions::quadratic::QuadraticExtensionFieldElement},
    traits::ByteConversion,
    unsigned_integer::element::U256 as LambdaWorksU256,
};
use num_bigint::BigUint;
use secp256k1::{ecdsa, Message, Secp256k1};
use sha3::{Digest, Keccak256};

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
    let gas_cost = RIPEMD_160_COST + ripemd_160_dynamic_cost(calldata.len() as u64);
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

/// Arbitrary-precision exponentiation under modulo.
/// More info in https://eips.ethereum.org/EIPS/eip-198 and https://www.evm.codes/precompiled.
/// Arbitrary-precision exponentiation under modulo.
/// More info in https://eips.ethereum.org/EIPS/eip-198 and https://www.evm.codes/precompiled.
pub fn modexp(
    calldata: &Bytes,
    gas_limit: u64,
    consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    if calldata.is_empty() {
        *consumed_gas += MIN_MODEXP_COST;
        return Ok(Bytes::new());
    }

    let calldata = right_pad(calldata, MXP_PARAMS_OFFSET);

    // Cast sizes as usize and check for overflow.
    // Bigger sizes are not accepted, as memory can't index bigger values.
    let b_size = usize::try_from(U256::from_big_endian(&calldata[..BSIZE_END]))
        .map_err(|_| PrecompileError::InvalidCalldata)?;
    let e_size = usize::try_from(U256::from_big_endian(&calldata[BSIZE_END..ESIZE_END]))
        .map_err(|_| PrecompileError::InvalidCalldata)?;
    let m_size = usize::try_from(U256::from_big_endian(&calldata[ESIZE_END..MSIZE_END]))
        .map_err(|_| PrecompileError::InvalidCalldata)?;

    // Handle special case when both the base and mod are zero.
    if b_size == 0 && m_size == 0 {
        *consumed_gas += 200;
        return Ok(Bytes::new());
    }

    let params_len = MXP_PARAMS_OFFSET + b_size + e_size + m_size;
    if calldata.len() < params_len {
        return Err(PrecompileError::InvalidCalldata);
    }

    let b = BigUint::from_bytes_be(&calldata[MXP_PARAMS_OFFSET..MXP_PARAMS_OFFSET + b_size]);
    let e = BigUint::from_bytes_be(
        &calldata[MXP_PARAMS_OFFSET + b_size..MXP_PARAMS_OFFSET + b_size + e_size],
    );
    let m = BigUint::from_bytes_be(&calldata[MXP_PARAMS_OFFSET + b_size + e_size..params_len]);

    // Compute gas cost
    let max_length = b_size.max(m_size);
    let words = (max_length + 7) / 8;
    let multiplication_complexity = (words * words) as u64;
    let iteration_count = if e_size <= 32 && e != BigUint::ZERO {
        e.bits() - 1
    } else if e_size > 32 {
        8 * (e_size as u64 - 32) + e.bits().max(1) - 1
    } else {
        0
    };
    let calculate_iteration_count = iteration_count.max(1);
    let gas_cost = (multiplication_complexity * calculate_iteration_count / 3).max(MIN_MODEXP_COST);
    if gas_limit < gas_cost {
        return Err(PrecompileError::NotEnoughGas);
    }
    *consumed_gas += gas_cost;

    let result = if m == BigUint::ZERO {
        BigUint::ZERO
    } else if e == BigUint::ZERO {
        BigUint::from(1_u8) % m
    } else {
        b.modpow(&e, &m)
    };
    let result = left_pad(&Bytes::from(result.to_bytes_be()), m_size);
    Ok(result.slice(..m_size))
}

/// Point addition on the elliptic curve 'alt_bn128' (also referred as 'bn254').
/// More info in https://eips.ethereum.org/EIPS/eip-196 and https://www.evm.codes/precompiled.
pub fn ecadd(
    calldata: &Bytes,
    gas_limit: u64,
    consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    if gas_limit < ECADD_COST {
        return Err(PrecompileError::NotEnoughGas);
    }

    let calldata = right_pad(calldata, ECADD_PARAMS_OFFSET);
    // Slice lengths are checked, so unwrap is safe
    let x1 = BN254FieldElement::from_bytes_be(&calldata[..ECADD_X1_END]).unwrap();
    let y1 = BN254FieldElement::from_bytes_be(&calldata[ECADD_X1_END..ECADD_Y1_END]).unwrap();
    let x2 = BN254FieldElement::from_bytes_be(&calldata[ECADD_Y1_END..ECADD_X2_END]).unwrap();
    let y2 = BN254FieldElement::from_bytes_be(&calldata[ECADD_X2_END..ECADD_Y2_END]).unwrap();

    // (0,0) represents infinity, in that case the other point (if valid) should be directly returned
    let zero_el = BN254FieldElement::from(0);
    let p1_is_infinity = x1.eq(&zero_el) && y1.eq(&zero_el);
    let p2_is_infinity = x2.eq(&zero_el) && y2.eq(&zero_el);

    match (p1_is_infinity, p2_is_infinity) {
        (true, true) => {
            *consumed_gas += ECADD_COST;
            return Ok(Bytes::from([0u8; 64].to_vec()));
        }
        (true, false) => {
            if let Ok(p2) = BN254Curve::create_point_from_affine(x2, y2) {
                *consumed_gas += ECADD_COST;
                let res = [p2.x().to_bytes_be(), p2.y().to_bytes_be()].concat();
                return Ok(Bytes::from(res));
            }
            return Err(PrecompileError::InvalidEcPoint);
        }
        (false, true) => {
            if let Ok(p1) = BN254Curve::create_point_from_affine(x1, y1) {
                *consumed_gas += ECADD_COST;
                let res = [p1.x().to_bytes_be(), p1.y().to_bytes_be()].concat();
                return Ok(Bytes::from(res));
            }
            return Err(PrecompileError::InvalidEcPoint);
        }
        _ => {}
    }

    let (Ok(p1), Ok(p2)) = (
        BN254Curve::create_point_from_affine(x1, y1),
        BN254Curve::create_point_from_affine(x2, y2),
    ) else {
        return Err(PrecompileError::InvalidEcPoint);
    };

    *consumed_gas += ECADD_COST;
    let sum = p1.operate_with(&p2).to_affine();
    let res = [sum.x().to_bytes_be(), sum.y().to_bytes_be()].concat();
    Ok(Bytes::from(res))
}

/// Scalar multiplication on the elliptic curve 'alt_bn128' (also referred as 'bn254').
/// More info in https://eips.ethereum.org/EIPS/eip-196 and https://www.evm.codes/precompiled.
pub fn ecmul(
    calldata: &Bytes,
    gas_limit: u64,
    consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    if gas_limit < ECMUL_COST {
        return Err(PrecompileError::NotEnoughGas);
    }

    let calldata = right_pad(calldata, ECMUL_PARAMS_OFFSET);
    // Slice lengths are checked, so unwrap is safe
    let x1 = BN254FieldElement::from_bytes_be(&calldata[..ECMUL_X1_END]).unwrap();
    let y1 = BN254FieldElement::from_bytes_be(&calldata[ECMUL_X1_END..ECMUL_Y1_END]).unwrap();
    let s = LambdaWorksU256::from_bytes_be(&calldata[ECMUL_Y1_END..ECMUL_S_END]).unwrap();

    // if the point is infinity it is directly returned
    let zero_el = BN254FieldElement::from(0);
    let p1_is_infinity = x1.eq(&zero_el) && y1.eq(&zero_el);
    if p1_is_infinity {
        *consumed_gas += ECMUL_COST;
        return Ok(Bytes::from([0u8; 64].to_vec()));
    }

    // scalar is 0 and the point is valid
    let zero_u256 = LambdaWorksU256::from(0_u16);
    if s.eq(&zero_u256) && BN254Curve::create_point_from_affine(x1.clone(), y1.clone()).is_ok() {
        *consumed_gas += ECMUL_COST;
        return Ok(Bytes::from([0u8; 64].to_vec()));
    }

    if let Ok(p1) = BN254Curve::create_point_from_affine(x1, y1) {
        *consumed_gas += ECMUL_COST;
        let mul = p1.operate_with_self(s).to_affine();
        let res = [mul.x().to_bytes_be(), mul.y().to_bytes_be()].concat();
        return Ok(Bytes::from(res));
    }

    Err(PrecompileError::InvalidEcPoint)
}

/// Elliptic curve pairing operation required in order to perform zkSNARK verification within the block gas limit. Bilinear function on groups on the elliptic curve “alt_bn128”.
/// More info in https://eips.ethereum.org/EIPS/eip-197 and https://www.evm.codes/precompiled.
/// - Loops over the calldata in chunks of 192 bytes. K times, where K = len / 192.
/// - Two groups G_1 and G_2, which sum up to 192 bytes.
/// - With each field size being 32 bytes.
pub fn ecpairing(
    calldata: &Bytes,
    gas_limit: u64,
    consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    if calldata.len() % ECP_INPUT_SIZE != 0 {
        return Err(PrecompileError::InvalidCalldata);
    }
    let gas_cost = ECPAIRING_STATIC_COST + ecpairing_dynamic_cost(calldata.len() as u64);
    if gas_limit < gas_cost {
        return Err(PrecompileError::NotEnoughGas);
    }

    let rounds = calldata.len() / ECP_INPUT_SIZE;
    let mut mul: FieldElement<Degree12ExtensionField> = QuadraticExtensionFieldElement::one();
    for idx in 0..rounds {
        let start = idx * ECP_INPUT_SIZE;

        // Slice lengths are checked, so unwrap is safe
        let g1_x =
            BN254FieldElement::from_bytes_be(&calldata[start..start + ECP_FIELD_SIZE]).unwrap();
        let g1_y = BN254FieldElement::from_bytes_be(
            &calldata[start + ECP_FIELD_SIZE..start + double_field_size()],
        )
        .unwrap();

        let g2_x_bytes = [
            &calldata[start + ecpairing_g2_point1_start(G1_POINT_POS)
                ..start + ecpairing_g2_point1_end(G1_POINT_POS)], // calldata[start + 96..start + 128]
            &calldata[start + G1_POINT_POS..start + ecpairing_g2_point1_start(G1_POINT_POS)], // calldata[start + 64..start + 96]
        ]
        .concat();
        let g2_y_bytes = [
            &calldata[start + ecpairing_g2_point1_start(G2_POINT_POS)
                ..start + ecpairing_g2_point1_end(G2_POINT_POS)], // calldata[start + 160..start + 192]
            &calldata[start + G2_POINT_POS..start + ecpairing_g2_point1_start(G2_POINT_POS)], // calldata[start + 128..start + 160]
        ]
        .concat();

        let g2_x = BN254TwistCurveFieldElement::from_bytes_be(&g2_x_bytes);
        let g2_y = BN254TwistCurveFieldElement::from_bytes_be(&g2_y_bytes);

        let (Ok(g2_x), Ok(g2_y)) = (g2_x, g2_y) else {
            return Err(PrecompileError::InvalidEcPoint);
        };

        // if any point is (0,0) the pairing is ok
        let zero_el = BN254FieldElement::from(0);
        let tw_zero_el = BN254TwistCurveFieldElement::from(0);
        let p1_is_infinity = g1_x.eq(&zero_el) && g1_y.eq(&zero_el);
        let p2_is_infinity = g2_x.eq(&tw_zero_el) && g2_y.eq(&tw_zero_el);
        if p1_is_infinity || p2_is_infinity {
            continue;
        }

        let (Ok(p1), Ok(p2)) = (
            BN254Curve::create_point_from_affine(g1_x, g1_y),
            BN254TwistCurve::create_point_from_affine(g2_x, g2_y),
        ) else {
            return Err(PrecompileError::InvalidEcPoint);
        };

        let Ok(pairing_result) = BN254AtePairing::compute_batch(&[(&p1, &p2)]) else {
            return Err(PrecompileError::InvalidEcPoint);
        };
        mul *= pairing_result;
    }

    *consumed_gas += gas_cost;
    let success = mul.eq(&QuadraticExtensionFieldElement::one());
    let mut output = vec![0_u8; 32];
    output[31] = success as u8;
    Ok(Bytes::from(output))
}

// Extracted from https://datatracker.ietf.org/doc/html/rfc7693#section-2.7
pub const SIGMA: [[usize; 16]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
];

// Extracted from https://datatracker.ietf.org/doc/html/rfc7693#appendix-C.2
pub const IV: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];

// Extracted from https://datatracker.ietf.org/doc/html/rfc7693#section-2.1
const R1: u32 = 32;
const R2: u32 = 24;
const R3: u32 = 16;
const R4: u32 = 63;

// Based on https://datatracker.ietf.org/doc/html/rfc7693#section-3.1
fn g(v: &mut [u64; 16], a: usize, b: usize, c: usize, d: usize, x: u64, y: u64) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x); //mod 64 operations
    v[d] = (v[d] ^ v[a]).rotate_right(R1); // >>> operation
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(R2);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = (v[d] ^ v[a]).rotate_right(R3);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = (v[b] ^ v[c]).rotate_right(R4);
}

// Based on https://datatracker.ietf.org/doc/html/rfc7693#section-3.2
fn blake2f_compress(rounds: usize, h: &mut [u64; 8], m: &[u64; 16], t: &[u64; 2], f: bool) {
    // Initialize local work vector v[0..15]
    let mut v: [u64; 16] = [0_u64; 16];
    v[0..8].copy_from_slice(h); // First half from state
    v[8..16].copy_from_slice(&IV); // Second half from IV

    v[12] ^= t[0]; // Low word of the offset
    v[13] ^= t[1]; // High word of the offset

    if f {
        v[14] = !v[14]; // Invert all bits
    }

    for i in 0..rounds {
        // Message word selection permutation for this round
        let s: &[usize; 16] = &SIGMA[i % 10];

        g(&mut v, 0, 4, 8, 12, m[s[0]], m[s[1]]);
        g(&mut v, 1, 5, 9, 13, m[s[2]], m[s[3]]);
        g(&mut v, 2, 6, 10, 14, m[s[4]], m[s[5]]);
        g(&mut v, 3, 7, 11, 15, m[s[6]], m[s[7]]);

        g(&mut v, 0, 5, 10, 15, m[s[8]], m[s[9]]);
        g(&mut v, 1, 6, 11, 12, m[s[10]], m[s[11]]);
        g(&mut v, 2, 7, 8, 13, m[s[12]], m[s[13]]);
        g(&mut v, 3, 4, 9, 14, m[s[14]], m[s[15]]);
    }

    // XOR the two halves
    for i in 0..8 {
        h[i] = h[i] ^ v[i] ^ v[i + 8];
    }
}

const CALLDATA_LEN: usize = 213;

/// Compression function F used in the BLAKE2 cryptographic hashing algorithm.
/// More info in https://eips.ethereum.org/EIPS/eip-152 and https://www.evm.codes/precompiled.
/// Compression function F used in the BLAKE2 cryptographic hashing algorithm.
/// More info in https://eips.ethereum.org/EIPS/eip-152 and https://www.evm.codes/precompiled.
pub fn blake2f(
    calldata: &Bytes,
    gas_limit: u64,
    consumed_gas: &mut u64,
) -> Result<Bytes, PrecompileError> {
    /*
    [0; 3] (4 bytes)	rounds	Number of rounds (big-endian unsigned integer)
    [4; 67] (64 bytes)	h	State vector (8 8-byte little-endian unsigned integer)
    [68; 195] (128 bytes)	m	Message block vector (16 8-byte little-endian unsigned integer)
    [196; 211] (16 bytes)	t	Offset counters (2 8-byte little-endian integer)
    [212; 212] (1 byte)	f	Final block indicator flag (0 or 1)
    */

    if calldata.len() != CALLDATA_LEN {
        return Err(PrecompileError::InvalidCalldata);
    }

    let rounds = u32::from_be_bytes(
        calldata[..BF2_ROUND_END]
            .try_into()
            .map_err(|_| PrecompileError::InvalidCalldata)?,
    );

    let needed_gas = blake2_gas_cost(rounds);
    if needed_gas > gas_limit {
        return Err(PrecompileError::NotEnoughGas);
    }

    let mut h: [u64; 8] = [0_u64; 8];
    let mut m: [u64; 16] = [0_u64; 16];
    let mut t: [u64; 2] = [0_u64; 2];
    let f = u8::from_be_bytes(
        calldata[BF2_BLOCK_FLAG..(BF2_BLOCK_FLAG + 1)]
            .try_into()
            .map_err(|_| PrecompileError::InvalidCalldata)?,
    );

    if f > 1 {
        return Err(PrecompileError::InvalidCalldata);
    }
    let f = f == 1;

    // NOTE: We may optimize this by unwraping both for loops

    for (i, h) in h.iter_mut().enumerate() {
        let start = BF2_STATEVEC_INIT + i * BF2_VEC_ELEM_SIZE;
        *h = u64::from_le_bytes(
            calldata[start..start + BF2_VEC_ELEM_SIZE]
                .try_into()
                .map_err(|_| PrecompileError::InvalidCalldata)?,
        );
    }

    for (i, m) in m.iter_mut().enumerate() {
        let start = BF2_MSGVEC_INIT + i * BF2_VEC_ELEM_SIZE;
        *m = u64::from_le_bytes(
            calldata[start..start + BF2_VEC_ELEM_SIZE]
                .try_into()
                .map_err(|_| PrecompileError::InvalidCalldata)?,
        );
    }

    t[0] = u64::from_le_bytes(
        calldata[BF2_OFFSET_COUNT_INIT..BF2_OFFSET_COUNT_INIT + BF2_VEC_ELEM_SIZE]
            .try_into()
            .map_err(|_| PrecompileError::InvalidCalldata)?,
    );
    t[1] = u64::from_le_bytes(
        calldata[BF2_OFFSET_COUNT_INIT + BF2_VEC_ELEM_SIZE..BF2_BLOCK_FLAG]
            .try_into()
            .map_err(|_| PrecompileError::InvalidCalldata)?,
    );

    blake2f_compress(rounds as _, &mut h, &m, &t, f);
    *consumed_gas += needed_gas;

    let out: Vec<u8> = h.iter().flat_map(|&num| num.to_le_bytes()).collect();
    Ok(Bytes::from(out))
}

pub fn is_precompile(callee_address: Address) -> bool {
    let addr_as_u64 = callee_address.to_low_u64_be();
    // TODO: replace 10 with point evaluation address constant and include it in the range (..=10)
    callee_address[0..12] == [0u8; 12] && (ECRECOVER_ADDRESS..10).contains(&addr_as_u64)
}

pub fn execute_precompile(
    callee_address: Address,
    calldata: Bytes,
    gas_to_send: u64,
    consumed_gas: &mut u64,
) -> (u8, Bytes) {
    let result = match callee_address {
        x if x == Address::from_low_u64_be(ECRECOVER_ADDRESS) => {
            ecrecover(&calldata, gas_to_send, consumed_gas)
        }
        x if x == Address::from_low_u64_be(IDENTITY_ADDRESS) => {
            identity(&calldata, gas_to_send, consumed_gas)
        }
        x if x == Address::from_low_u64_be(SHA2_256_ADDRESS) => {
            sha2_256(&calldata, gas_to_send, consumed_gas)
        }
        x if x == Address::from_low_u64_be(RIPEMD_160_ADDRESS) => {
            ripemd_160(&calldata, gas_to_send, consumed_gas)
        }
        x if x == Address::from_low_u64_be(MODEXP_ADDRESS) => {
            modexp(&calldata, gas_to_send, consumed_gas)
        }
        x if x == Address::from_low_u64_be(ECADD_ADDRESS) => {
            ecadd(&calldata, gas_to_send, consumed_gas)
        }
        x if x == Address::from_low_u64_be(ECMUL_ADDRESS) => {
            ecmul(&calldata, gas_to_send, consumed_gas)
        }
        x if x == Address::from_low_u64_be(ECPAIRING_ADDRESS) => {
            ecpairing(&calldata, gas_to_send, consumed_gas)
        }
        x if x == Address::from_low_u64_be(BLAKE2F_ADDRESS) => {
            blake2f(&calldata, gas_to_send, consumed_gas)
        }
        _ => {
            unreachable!()
        }
    };
    match result {
        Ok(res) => (SUCCESS_RETURN_CODE, res),
        Err(_) => {
            *consumed_gas += gas_to_send;
            (REVERT_RETURN_CODE, Bytes::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calldata_for_modexp(b_size: u16, e_size: u16, m_size: u16, b: u8, e: u8, m: u8) -> Bytes {
        let calldata_size = (b_size + e_size + m_size + MXP_PARAMS_OFFSET as u16) as usize;
        let b_data_size = U256::from(b_size);
        let e_data_size = U256::from(e_size);
        let m_data_size = U256::from(m_size);
        let e_size = e_size as usize;
        let m_size = m_size as usize;

        let mut calldata = vec![0_u8; calldata_size];
        let calldata_slice = calldata.as_mut_slice();
        b_data_size.to_big_endian(&mut calldata_slice[..BSIZE_END]);
        e_data_size.to_big_endian(&mut calldata_slice[BSIZE_END..ESIZE_END]);
        m_data_size.to_big_endian(&mut calldata_slice[ESIZE_END..MSIZE_END]);
        calldata_slice[calldata_size - m_size - e_size - 1] = b;
        calldata_slice[calldata_size - m_size - 1] = e;
        calldata_slice[calldata_size - 1] = m;

        Bytes::from(calldata_slice.to_vec())
    }

    #[test]
    fn modexp_min_gas_cost() {
        let callee_address = Address::from_low_u64_be(MODEXP_ADDRESS);
        let calldata = calldata_for_modexp(1, 1, 1, 8, 9, 10);
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, Bytes::from(8_u8.to_be_bytes().to_vec()));
        assert_eq!(consumed_gas, MIN_MODEXP_COST);
    }

    #[test]
    fn modexp_variable_gas_cost() {
        let callee_address = Address::from_low_u64_be(MODEXP_ADDRESS);
        let calldata = calldata_for_modexp(256, 1, 1, 8, 6, 10);
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, Bytes::from(4_u8.to_be_bytes().to_vec()));
        assert_eq!(consumed_gas, 682);
    }

    #[test]
    fn modexp_not_enought_gas() {
        let callee_address = Address::from_low_u64_be(MODEXP_ADDRESS);
        let calldata = calldata_for_modexp(1, 1, 1, 8, 9, 10);
        let gas_limit = 199;
        let mut consumed_gas = 0;

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, REVERT_RETURN_CODE);
        assert_eq!(return_data, Bytes::new());
        assert_eq!(consumed_gas, gas_limit);
    }

    #[test]
    fn modexp_zero_modulo() {
        let callee_address = Address::from_low_u64_be(MODEXP_ADDRESS);
        let calldata = calldata_for_modexp(1, 1, 1, 8, 9, 0);
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, Bytes::from(0_u8.to_be_bytes().to_vec()));
        assert_eq!(consumed_gas, MIN_MODEXP_COST);
    }

    #[test]
    fn modexp_bigger_msize_than_necessary() {
        let callee_address = Address::from_low_u64_be(MODEXP_ADDRESS);
        let calldata = calldata_for_modexp(1, 1, 32, 8, 6, 10);
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        let mut expected_return_data = 4_u8.to_be_bytes().to_vec();
        expected_return_data.resize(32, 0);
        expected_return_data.reverse();
        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, Bytes::from(expected_return_data));
    }

    #[test]
    fn modexp_big_sizes_for_values() {
        let callee_address = Address::from_low_u64_be(MODEXP_ADDRESS);
        let calldata = calldata_for_modexp(256, 255, 255, 8, 6, 10);
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        let mut expected_return_data = 4_u8.to_be_bytes().to_vec();
        expected_return_data.resize(255, 0);
        expected_return_data.reverse();
        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, Bytes::from(expected_return_data));
    }

    #[test]
    fn modexp_with_empty_calldata() {
        let callee_address = Address::from_low_u64_be(MODEXP_ADDRESS);
        let calldata = Bytes::new();
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, Bytes::new());
        assert_eq!(consumed_gas, MIN_MODEXP_COST);
    }

    #[test]
    fn ecadd_happy_path() {
        let callee_address = Address::from_low_u64_be(ECADD_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000002\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let expected_x =
            hex::decode("030644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd3")
                .unwrap();
        let expected_y =
            hex::decode("15ed738c0e0a7c92e7845f96b2ae9c0a68a6a449e3538fc7ff3ebf7a5a18a2c4")
                .unwrap();
        let expected_result = Bytes::from([expected_x, expected_y].concat());

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, expected_result);
        assert_eq!(consumed_gas, ECADD_COST);
    }

    #[test]
    fn ecadd_infinity_with_valid_point() {
        let callee_address = Address::from_low_u64_be(ECADD_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let expected_x =
            hex::decode("0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap();
        let expected_y =
            hex::decode("0000000000000000000000000000000000000000000000000000000000000002")
                .unwrap();
        let expected_result = Bytes::from([expected_x, expected_y].concat());

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, expected_result);
        assert_eq!(consumed_gas, ECADD_COST);
    }

    #[test]
    fn ecadd_valid_point_with_infinity() {
        let callee_address = Address::from_low_u64_be(ECADD_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000002\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let expected_x =
            hex::decode("0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap();
        let expected_y =
            hex::decode("0000000000000000000000000000000000000000000000000000000000000002")
                .unwrap();
        let expected_result = Bytes::from([expected_x, expected_y].concat());

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, expected_result);
        assert_eq!(consumed_gas, ECADD_COST);
    }

    #[test]
    fn ecadd_infinity_twice() {
        let callee_address = Address::from_low_u64_be(ECADD_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, Bytes::from([0u8; 64].to_vec()));
        assert_eq!(consumed_gas, ECADD_COST);
    }

    #[test]
    fn ecadd_with_empty_calldata() {
        let callee_address = Address::from_low_u64_be(ECADD_ADDRESS);
        let calldata = Bytes::new();
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, Bytes::from([0u8; 64].to_vec()));
        assert_eq!(consumed_gas, ECADD_COST);
    }

    #[test]
    fn ecadd_with_invalid_first_point() {
        let callee_address = Address::from_low_u64_be(ECADD_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let result = ecadd(&calldata, gas_limit, &mut consumed_gas);
        // just to check error type, no gas consumption made
        assert_eq!(result.unwrap_err(), PrecompileError::InvalidEcPoint);

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, REVERT_RETURN_CODE);
        assert_eq!(return_data, Bytes::new());
        assert_eq!(consumed_gas, gas_limit);
    }

    #[test]
    fn ecadd_with_invalid_second_point() {
        let callee_address = Address::from_low_u64_be(ECADD_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000002\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let result = ecadd(&calldata, gas_limit, &mut consumed_gas);
        // just to check error type, no gas consumption made
        assert_eq!(result.unwrap_err(), PrecompileError::InvalidEcPoint);

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, REVERT_RETURN_CODE);
        assert_eq!(return_data, Bytes::new());
        assert_eq!(consumed_gas, gas_limit);
    }

    #[test]
    fn ecadd_with_not_enough_gas() {
        let callee_address = Address::from_low_u64_be(ECADD_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000002\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
        );
        let gas_limit = ECADD_COST - 1;
        let mut consumed_gas = 0;

        let result = ecadd(&calldata, gas_limit, &mut consumed_gas);
        // just to check error type, no gas consumption made
        assert_eq!(result.unwrap_err(), PrecompileError::NotEnoughGas);

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, REVERT_RETURN_CODE);
        assert_eq!(return_data, Bytes::new());
        assert_eq!(consumed_gas, gas_limit);
    }

    #[test]
    fn ecmul_happy_path() {
        let callee_address = Address::from_low_u64_be(ECMUL_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000002\
            0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let expected_x =
            hex::decode("030644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd3")
                .unwrap();
        let expected_y =
            hex::decode("15ed738c0e0a7c92e7845f96b2ae9c0a68a6a449e3538fc7ff3ebf7a5a18a2c4")
                .unwrap();
        let expected_result = Bytes::from([expected_x, expected_y].concat());

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, expected_result);
        assert_eq!(consumed_gas, ECMUL_COST);
    }

    #[test]
    fn ecmul_infinity() {
        let callee_address = Address::from_low_u64_be(ECMUL_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, Bytes::from([0u8; 64].to_vec()));
        assert_eq!(consumed_gas, ECMUL_COST);
    }

    #[test]
    fn ecmul_by_zero() {
        let callee_address = Address::from_low_u64_be(ECMUL_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000002\
            0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, Bytes::from([0u8; 64].to_vec()));
        assert_eq!(consumed_gas, ECMUL_COST);
    }

    #[test]
    fn ecmul_with_empty_calldata() {
        let callee_address = Address::from_low_u64_be(ECMUL_ADDRESS);
        let calldata = Bytes::new();
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, Bytes::from([0u8; 64].to_vec()));
        assert_eq!(consumed_gas, ECMUL_COST);
    }

    #[test]
    fn ecmul_invalid_point() {
        let callee_address = Address::from_low_u64_be(ECMUL_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let result = ecmul(&calldata, gas_limit, &mut consumed_gas);
        // just to check error type, no gas consumption made
        assert_eq!(result.unwrap_err(), PrecompileError::InvalidEcPoint);

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, REVERT_RETURN_CODE);
        assert_eq!(return_data, Bytes::new());
        assert_eq!(consumed_gas, gas_limit);
    }

    #[test]
    fn ecmul_invalid_point_by_zero() {
        let callee_address = Address::from_low_u64_be(ECMUL_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let result = ecmul(&calldata, gas_limit, &mut consumed_gas);
        // just to check error type, no gas consumption made
        assert_eq!(result.unwrap_err(), PrecompileError::InvalidEcPoint);

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, REVERT_RETURN_CODE);
        assert_eq!(return_data, Bytes::new());
        assert_eq!(consumed_gas, gas_limit);
    }

    #[test]
    fn ecmul_with_not_enough_gas() {
        let callee_address = Address::from_low_u64_be(ECMUL_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
            0000000000000000000000000000000000000000000000000000000000000001\
            0000000000000000000000000000000000000000000000000000000000000002\
            0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
        );
        let gas_limit = ECMUL_COST - 1;
        let mut consumed_gas = 0;

        let result = ecmul(&calldata, gas_limit, &mut consumed_gas);
        // just to check error type, no gas consumption made
        assert_eq!(result.unwrap_err(), PrecompileError::NotEnoughGas);

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, REVERT_RETURN_CODE);
        assert_eq!(return_data, Bytes::new());
        assert_eq!(consumed_gas, gas_limit);
    }

    #[test]
    fn ecpairing_happy_path() {
        let callee_address = Address::from_low_u64_be(ECPAIRING_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
            2cf44499d5d27bb186308b7af7af02ac5bc9eeb6a3d147c186b21fb1b76e18da\
            2c0f001f52110ccfe69108924926e45f0b0c868df0e7bde1fe16d3242dc715f6\
            1fb19bb476f6b9e44e2a32234da8212f61cd63919354bc06aef31e3cfaff3ebc\
            22606845ff186793914e03e21df544c34ffe2f2f3504de8a79d9159eca2d98d9\
            2bd368e28381e8eccb5fa81fc26cf3f048eea9abfdd85d7ed3ab3698d63e4f90\
            2fe02e47887507adf0ff1743cbac6ba291e66f59be6bd763950bb16041a0a85e\
            0000000000000000000000000000000000000000000000000000000000000001\
            30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd45\
            1971ff0471b09fa93caaf13cbf443c1aede09cc4328f5a62aad45f40ec133eb4\
            091058a3141822985733cbdddfed0fd8d6c104e9e9eff40bf5abfef9ab163bc7\
            2a23af9a5ce2ba2796c1f4e453a370eb0af8c212d9dc9acd8fc02c2e907baea2\
            23a8eb0b0996252cb548a4487da97b02422ebc0e834613f954de6c7e0afdc1fc",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let expected_result = Bytes::from(
            hex::decode("0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap(),
        );

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, expected_result);
        assert_eq!(consumed_gas, 113_000);
    }

    #[test]
    fn ecpairing_p1_is_infinity() {
        let callee_address = Address::from_low_u64_be(ECPAIRING_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
                0000000000000000000000000000000000000000000000000000000000000000\
                0000000000000000000000000000000000000000000000000000000000000000\
                1fb19bb476f6b9e44e2a32234da8212f61cd63919354bc06aef31e3cfaff3ebc\
                22606845ff186793914e03e21df544c34ffe2f2f3504de8a79d9159eca2d98d9\
                2bd368e28381e8eccb5fa81fc26cf3f048eea9abfdd85d7ed3ab3698d63e4f90\
                2fe02e47887507adf0ff1743cbac6ba291e66f59be6bd763950bb16041a0a85e",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let expected_result = Bytes::from(
            hex::decode("0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap(),
        );

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, expected_result);
        assert_eq!(consumed_gas, 79_000);
    }

    #[test]
    fn ecpairing_p2_is_infinity() {
        let callee_address = Address::from_low_u64_be(ECPAIRING_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
                2cf44499d5d27bb186308b7af7af02ac5bc9eeb6a3d147c186b21fb1b76e18da\
                2c0f001f52110ccfe69108924926e45f0b0c868df0e7bde1fe16d3242dc715f6\
                0000000000000000000000000000000000000000000000000000000000000000\
                0000000000000000000000000000000000000000000000000000000000000000\
                0000000000000000000000000000000000000000000000000000000000000000\
                0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let expected_result = Bytes::from(
            hex::decode("0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap(),
        );

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, expected_result);
        assert_eq!(consumed_gas, 79_000);
    }

    #[test]
    fn ecpairing_empty_calldata() {
        let callee_address = Address::from_low_u64_be(ECPAIRING_ADDRESS);
        let calldata = Bytes::new();
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let expected_result = Bytes::from(
            hex::decode("0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap(),
        );

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, expected_result);
        assert_eq!(consumed_gas, ECPAIRING_STATIC_COST);
    }

    #[test]
    fn ecpairing_invalid_point() {
        let callee_address = Address::from_low_u64_be(ECPAIRING_ADDRESS);
        // changed last byte from `fc` to `fd`
        let calldata = Bytes::from(
            hex::decode(
                "\
            2cf44499d5d27bb186308b7af7af02ac5bc9eeb6a3d147c186b21fb1b76e18da\
            2c0f001f52110ccfe69108924926e45f0b0c868df0e7bde1fe16d3242dc715f6\
            1fb19bb476f6b9e44e2a32234da8212f61cd63919354bc06aef31e3cfaff3ebc\
            22606845ff186793914e03e21df544c34ffe2f2f3504de8a79d9159eca2d98d9\
            2bd368e28381e8eccb5fa81fc26cf3f048eea9abfdd85d7ed3ab3698d63e4f90\
            2fe02e47887507adf0ff1743cbac6ba291e66f59be6bd763950bb16041a0a85e\
            0000000000000000000000000000000000000000000000000000000000000001\
            30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd45\
            1971ff0471b09fa93caaf13cbf443c1aede09cc4328f5a62aad45f40ec133eb4\
            091058a3141822985733cbdddfed0fd8d6c104e9e9eff40bf5abfef9ab163bc7\
            2a23af9a5ce2ba2796c1f4e453a370eb0af8c212d9dc9acd8fc02c2e907baea2\
            23a8eb0b0996252cb548a4487da97b02422ebc0e834613f954de6c7e0afdc1fd",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let result = ecpairing(&calldata, gas_limit, &mut consumed_gas);
        // just to check error type, no gas consumption made
        assert_eq!(result.unwrap_err(), PrecompileError::InvalidEcPoint);

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, REVERT_RETURN_CODE);
        assert_eq!(return_data, Bytes::new());
        assert_eq!(consumed_gas, gas_limit);
    }

    #[test]
    fn ecpairing_out_of_curve() {
        let callee_address = Address::from_low_u64_be(ECPAIRING_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
            1111111111111111111111111111111111111111111111111111111111111111\
            1111111111111111111111111111111111111111111111111111111111111111\
            1111111111111111111111111111111111111111111111111111111111111111\
            1111111111111111111111111111111111111111111111111111111111111111\
            1111111111111111111111111111111111111111111111111111111111111111\
            1111111111111111111111111111111111111111111111111111111111111111",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let result = ecpairing(&calldata, gas_limit, &mut consumed_gas);
        // just to check error type, no gas consumption made
        assert_eq!(result.unwrap_err(), PrecompileError::InvalidEcPoint);

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, REVERT_RETURN_CODE);
        assert_eq!(return_data, Bytes::new());
        assert_eq!(consumed_gas, gas_limit);
    }

    #[test]
    fn ecpairing_invalid_calldata() {
        let callee_address = Address::from_low_u64_be(ECPAIRING_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
                1111111111111111111111111111111111111111111111111111111111111111",
            )
            .unwrap(),
        );
        let gas_limit = 100_000_000;
        let mut consumed_gas = 0;

        let result = ecpairing(&calldata, gas_limit, &mut consumed_gas);
        // just to check error type, no gas consumption made
        assert_eq!(result.unwrap_err(), PrecompileError::InvalidCalldata);

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, REVERT_RETURN_CODE);
        assert_eq!(return_data, Bytes::new());
        assert_eq!(consumed_gas, gas_limit);
    }

    #[test]
    fn ecpairing_with_not_enough_gas() {
        let callee_address = Address::from_low_u64_be(ECPAIRING_ADDRESS);
        let calldata = Bytes::from(
            hex::decode(
                "\
            2cf44499d5d27bb186308b7af7af02ac5bc9eeb6a3d147c186b21fb1b76e18da\
            2c0f001f52110ccfe69108924926e45f0b0c868df0e7bde1fe16d3242dc715f6\
            1fb19bb476f6b9e44e2a32234da8212f61cd63919354bc06aef31e3cfaff3ebc\
            22606845ff186793914e03e21df544c34ffe2f2f3504de8a79d9159eca2d98d9\
            2bd368e28381e8eccb5fa81fc26cf3f048eea9abfdd85d7ed3ab3698d63e4f90\
            2fe02e47887507adf0ff1743cbac6ba291e66f59be6bd763950bb16041a0a85e\
            0000000000000000000000000000000000000000000000000000000000000001\
            30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd45\
            1971ff0471b09fa93caaf13cbf443c1aede09cc4328f5a62aad45f40ec133eb4\
            091058a3141822985733cbdddfed0fd8d6c104e9e9eff40bf5abfef9ab163bc7\
            2a23af9a5ce2ba2796c1f4e453a370eb0af8c212d9dc9acd8fc02c2e907baea2\
            23a8eb0b0996252cb548a4487da97b02422ebc0e834613f954de6c7e0afdc1fc",
            )
            .unwrap(),
        );
        // needs 113_000
        let gas_limit = 100_000;
        let mut consumed_gas = 0;

        let result = ecpairing(&calldata, gas_limit, &mut consumed_gas);
        // just to check error type, no gas consumption made
        assert_eq!(result.unwrap_err(), PrecompileError::NotEnoughGas);

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, REVERT_RETURN_CODE);
        assert_eq!(return_data, Bytes::new());
        assert_eq!(consumed_gas, gas_limit);
    }

    #[test]
    fn test_blake2_evm_codes_happy_path() {
        let callee_address = Address::from_low_u64_be(BLAKE2F_ADDRESS);
        let rounds = hex::decode("0000000c").unwrap();
        let h = hex::decode("48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b").unwrap();
        let m = hex::decode("6162630000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();
        let t = hex::decode("03000000000000000000000000000000").unwrap();
        let f = hex::decode("01").unwrap();
        let calldata = [rounds, h, m, t, f].concat();
        let calldata = Bytes::from(calldata);
        let gas_limit = 1000;
        let mut consumed_gas: u64 = 0;

        let expected_result = Bytes::from(hex::decode(
        "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923"
        ).unwrap());

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, expected_result);
        assert_eq!(consumed_gas, 12);
    }

    #[test]
    fn test_blake2_eip_example_1() {
        let callee_address = Address::from_low_u64_be(BLAKE2F_ADDRESS);
        let calldata = Bytes::from(hex::decode("00000c48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b61626300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000001").unwrap());
        let gas_limit = 1000;
        let mut consumed_gas: u64 = 0;

        let result = blake2f(&calldata, gas_limit, &mut consumed_gas);
        // just to check error type, no gas consumption made
        assert_eq!(result.unwrap_err(), PrecompileError::InvalidCalldata);

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, REVERT_RETURN_CODE);
        assert_eq!(return_data, Bytes::new());
        assert_eq!(consumed_gas, gas_limit);
    }

    #[test]
    fn test_blake2_eip_example_2() {
        let callee_address = Address::from_low_u64_be(BLAKE2F_ADDRESS);
        let calldata = Bytes::from(hex::decode("000000000c48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b61626300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000001").unwrap());
        let gas_limit = 1000;
        let mut consumed_gas: u64 = 0;

        let result = blake2f(&calldata, gas_limit, &mut consumed_gas);
        // just to check error type, no gas consumption made
        assert_eq!(result.unwrap_err(), PrecompileError::InvalidCalldata);

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, REVERT_RETURN_CODE);
        assert_eq!(return_data, Bytes::new());
        assert_eq!(consumed_gas, gas_limit);
    }

    #[test]
    fn test_blake2_eip_example_3() {
        let callee_address = Address::from_low_u64_be(BLAKE2F_ADDRESS);
        let calldata = Bytes::from(hex::decode("0000000c48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b61626300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000002").unwrap());
        let gas_limit = 1000;
        let mut consumed_gas: u64 = 0;

        let result = blake2f(&calldata, gas_limit, &mut consumed_gas);
        // just to check error type, no gas consumption made
        assert_eq!(result.unwrap_err(), PrecompileError::InvalidCalldata);

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, REVERT_RETURN_CODE);
        assert_eq!(return_data, Bytes::new());
        assert_eq!(consumed_gas, gas_limit);
    }

    #[test]
    fn test_blake2_eip_example_4() {
        let callee_address = Address::from_low_u64_be(BLAKE2F_ADDRESS);
        let calldata = Bytes::from(hex::decode("0000000048c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b61626300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000001").unwrap());
        let gas_limit = 1000;
        let mut consumed_gas: u64 = 0;

        let expected_result = Bytes::from(hex::decode(
        "08c9bcf367e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5d282e6ad7f520e511f6c3e2b8c68059b9442be0454267ce079217e1319cde05b"
        ).unwrap());

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, expected_result);
        assert_eq!(consumed_gas, 0);
    }

    #[test]
    fn test_blake2_example_5() {
        let callee_address = Address::from_low_u64_be(BLAKE2F_ADDRESS);
        let calldata = Bytes::from(hex::decode("0000000c48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b61626300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000001").unwrap());
        let gas_limit = 1000;
        let mut consumed_gas: u64 = 0;

        let expected_result = Bytes::from(hex::decode(
        "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923"
        ).unwrap());

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, expected_result);
        assert_eq!(consumed_gas, 12);
    }

    #[test]
    fn test_blake2_example_6() {
        let callee_address = Address::from_low_u64_be(BLAKE2F_ADDRESS);
        let calldata = Bytes::from(hex::decode("0000000c48c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b61626300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000").unwrap());
        let gas_limit = 1000;
        let mut consumed_gas: u64 = 0;

        let expected_result = Bytes::from(hex::decode(
        "75ab69d3190a562c51aef8d88f1c2775876944407270c42c9844252c26d2875298743e7f6d5ea2f2d3e8d226039cd31b4e426ac4f2d3d666a610c2116fde4735"
        ).unwrap());

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, expected_result);
        assert_eq!(consumed_gas, 12);
    }

    #[test]
    fn test_blake2_example_7() {
        let callee_address = Address::from_low_u64_be(BLAKE2F_ADDRESS);
        let calldata = Bytes::from(hex::decode("0000000148c9bdf267e6096a3ba7ca8485ae67bb2bf894fe72f36e3cf1361d5f3af54fa5d182e6ad7f520e511f6c3e2b8c68059b6bbd41fbabd9831f79217e1319cde05b61626300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000001").unwrap());
        let gas_limit = 1000;
        let mut consumed_gas: u64 = 0;

        let expected_result = Bytes::from(hex::decode(
        "b63a380cb2897d521994a85234ee2c181b5f844d2c624c002677e9703449d2fba551b3a8333bcdf5f2f7e08993d53923de3d64fcc68c034e717b9293fed7a421"
        ).unwrap());

        let (return_code, return_data) =
            execute_precompile(callee_address, calldata, gas_limit, &mut consumed_gas);

        assert_eq!(return_code, SUCCESS_RETURN_CODE);
        assert_eq!(return_data, expected_result);
        assert_eq!(consumed_gas, 1);
    }
}
