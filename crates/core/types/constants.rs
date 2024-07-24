use ethereum_types::H256;
use lazy_static::lazy_static;

// Fee related
pub const ELASTICITY_MULTIPLIER: u64 = 2;
pub const BASE_FEE_MAX_CHANGE_DENOMINATOR: u64 = 8;
pub const GAS_LIMIT_ADJUSTMENT_FACTOR: u64 = 1024;
pub const GAS_LIMIT_MINIMUM: u64 = 5000;
