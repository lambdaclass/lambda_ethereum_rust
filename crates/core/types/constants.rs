use ethereum_types::H256;
use lazy_static::lazy_static;
lazy_static! {
    pub static ref EMPTY_KECCACK_HASH: H256 = H256::from_slice(&hex::decode("1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347").unwrap()); // = Keccak256(RLP([])) as of EIP-3675
}

// Fee related
pub const ELASTICITY_MULTIPLIER: u64 = 2;
pub const BASE_FEE_MAX_CHANGE_DENOMINATOR: u64 = 8;
pub const GAS_LIMIT_ADJUSTMENT_FACTOR: u64 = 1024;
pub const GAS_LIMIT_MINIMUM: u64 = 5000;
