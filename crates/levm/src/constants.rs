pub const SUCCESS_FOR_CALL: i32 = 1;
pub const REVERT_FOR_CALL: i32 = 0;
pub const SUCCESS_FOR_RETURN: i32 = 1;

// Precompiled
// Ecrecover
pub const ECRECOVER_ADDRESS: u64 = 0x01;
pub const ECRECOVER_COST: u64 = 3000;
/// (0; 32) => Keccack-256 hash of the transaction.
pub const ECR_HASH_END: usize = 32;
/// The position of V in the signature.
pub const ECR_V_POS: usize = 63;
/// v ∈ {27, 28} => Recovery identifier, expected to be either 27 or 28.
pub const ECR_V_BASE: i32 = 27;
/// (64; 128) => signature, containing r and s.
pub const ECR_SIG_END: usize = 128;
pub const ECR_PARAMS_OFFSET: usize = 128;
/// The padding len is 12, as the return value is a publicAddress => the recovered 20-byte address right aligned to 32 bytes.
pub const ECR_PADDING_LEN: usize = 12;

// SHA2-256
pub const SHA2_256_STATIC_COST: u64 = 60;
pub const SHA2_256_ADDRESS: u64 = 0x02;

// Ripemd-160
pub const RIPEMD_160_STATIC_COST: u64 = 600;
pub const RIPEMD_160_ADDRESS: u64 = 0x03;
pub const RIPEMD_OUTPUT_LEN: usize = 32;
/// Used to align 32 bytes a 20-byte hash.
pub const RIPEMD_PADDING_LEN: usize = 12;

// Identity
pub const IDENTITY_STATIC_COST: u64 = 15;
pub const IDENTITY_ADDRESS: u64 = 0x04;

// modexp
pub const MODEXP_ADDRESS: u64 = 0x05;
pub const MIN_MODEXP_COST: u64 = 200;
/// (0; 32) contains byte size of B.
pub const BSIZE_END: usize = 32;
/// (32; 64) contains byte size of E.
pub const ESIZE_END: usize = 64;
/// (64; 96) contains byte size of M.
pub const MSIZE_END: usize = 96;
/// Used to get values of B, E and M.
pub const MXP_PARAMS_OFFSET: usize = 96;
