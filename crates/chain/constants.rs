// === EIP-4844 constants ===

/// Gas consumption of a single data blob (== blob byte size).
pub const GAS_PER_BLOB: u64 = 1 << 17;

/// Target number of the blob per block.
pub const TARGET_BLOB_NUMBER_PER_BLOCK: u64 = 3;

/// Max number of blobs per block
pub const MAX_BLOB_NUMBER_PER_BLOCK: u64 = 2 * TARGET_BLOB_NUMBER_PER_BLOCK;

/// Maximum consumable blob gas for data blobs per block.
pub const MAX_BLOB_GAS_PER_BLOCK: u64 = MAX_BLOB_NUMBER_PER_BLOCK * GAS_PER_BLOB;
