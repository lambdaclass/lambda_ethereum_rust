use ethereum_types::H160;

pub const DEFAULT_CONFIG_NAME: &str = "local";
pub const DEFAULT_L1_RPC_URL: &str = "http://localhost:8545";
pub const DEFAULT_L1_CHAIN_ID: u64 = 3151908;
pub const DEFAULT_L2_RPC_URL: &str = "http://localhost:1729";
pub const DEFAULT_L2_CHAIN_ID: u64 = 1729;
pub const DEFAULT_L2_EXPLORER_URL: &str = "";
pub const DEFAULT_L1_EXPLORER_URL: &str = "";
pub const DEFAULT_PRIVATE_KEY: &str =
    "0x850683b40d4a740aa6e745f889a6fdc8327be76e122f5aba645a5b02d0248db8";
// 0x5e6d086f5ec079adff4fb3774cdf3e8d6a34f7e9
pub const DEFAULT_ADDRESS: H160 = H160([
    0x5e, 0x6d, 0x08, 0x6f, 0x5e, 0xc0, 0x79, 0xad, 0xff, 0x4f, 0xb3, 0x77, 0x4c, 0xdf, 0x3e, 0x8d,
    0x6a, 0x34, 0xf7, 0xe9,
]);
