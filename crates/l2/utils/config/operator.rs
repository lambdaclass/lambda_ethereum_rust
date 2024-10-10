use crate::utils::secret_key_deserializer;
use ethereum_types::Address;
use libsecp256k1::SecretKey;
use serde::Deserialize;

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
        envy::prefixed("OPERATOR_")
            .from_env::<Self>()
            .map_err(ConfigError::from)
    }
}
