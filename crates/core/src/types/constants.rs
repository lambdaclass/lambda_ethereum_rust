// Transaction costs
pub const TX_BASE_COST: u64 = 21000;
pub const TX_DATA_COST_PER_NON_ZERO: u64 = 16;
pub const TX_DATA_COST_PER_ZERO: u64 = 4;
pub const TX_CREATE_COST: u64 = 32000;
pub const TX_ACCESS_LIST_ADDRESS_COST: u64 = 2400;
pub const TX_ACCESS_LIST_STORAGE_KEY_COST: u64 = 1900;
pub const MAX_CODE_SIZE: usize = 0x6000;
pub const GAS_INIT_CODE_WORD_COST: u64 = 2;
