use ethereum_types::{Address, H256};
use libsecp256k1::SecretKey;
use serde::{Deserialize, Deserializer};

use super::errors::ConfigError;

#[derive(Deserialize)]
pub struct OperatorConfig {
    pub block_executor_address: Address,
    pub operator_address: Address,
    #[serde(deserialize_with = "secret_key_deserializer")]
    pub operator_private_key: SecretKey,
    pub interval_ms: u64,
}

impl OperatorConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        envy::prefixed("XXX_")
            .from_env::<Self>()
            .map_err(ConfigError::from)
    }
}

fn secret_key_deserializer<'de, D>(deserializer: D) -> Result<SecretKey, D::Error>
where
    D: Deserializer<'de>,
{
    let hex = H256::deserialize(deserializer)?;
    SecretKey::parse(hex.as_fixed_bytes()).map_err(serde::de::Error::custom)
}
