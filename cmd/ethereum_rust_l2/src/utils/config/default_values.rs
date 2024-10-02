use ethereum_types::H160;

pub const DEFAULT_CONFIG_NAME: &str = "local";
pub const DEFAULT_L1_RPC_URL: &str = "http://localhost:8545";
pub const DEFAULT_L1_CHAIN_ID: u64 = 3151908;
pub const DEFAULT_L2_RPC_URL: &str = "http://localhost:1729";
pub const DEFAULT_L2_CHAIN_ID: u64 = 1729;
pub const DEFAULT_L2_EXPLORER_URL: &str = "";
pub const DEFAULT_L1_EXPLORER_URL: &str = "";
pub const DEFAULT_PRIVATE_KEY: &str =
    "0x385c546456b6a603a1cfcaa9ec9494ba4832da08dd6bcf4de9a71e4a01b74924";
// 0x3d1e15a1a55578f7c920884a9943b3b35d0d885b
pub const DEFAULT_ADDRESS: H160 = H160([
    0x3d, 0x1e, 0x15, 0xa1, 0xa5, 0x55, 0x78, 0xf7, 0xc9, 0x20, 0x88, 0x4a, 0x99, 0x43, 0xb3, 0xb3,
    0x5d, 0x0d, 0x88, 0x5b,
]);
