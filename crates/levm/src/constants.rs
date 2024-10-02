pub const SUCCESS_FOR_CALL: i32 = 1;
pub const REVERT_FOR_CALL: i32 = 0;
pub const SUCCESS_FOR_RETURN: i32 = 1;

// Precompiled
// Identity
pub const IDENTITY_STATIC_COST: u64 = 15;
pub const IDENTITY_ADDRESS: u64 = 0x04;

// SHA2-256
pub const SHA2_256_STATIC_COST: u64 = 60;
pub const SHA2_256_ADDRESS: u64 = 0x02;

// Ripemd-160
pub const RIPEMD_160_STATIC_COST: u64 = 600;
pub const RIPEMD_160_ADDRESS: u64 = 0x03;
pub const RIPEMD_OUTPUT_LEN: usize = 32;
/// Used to align 32 bytes a 20-byte hash.
pub const RIPEMD_PADDING_LEN: usize = 12;
