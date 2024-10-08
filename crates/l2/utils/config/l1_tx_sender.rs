use ethereum_types::{Address, H256};
use libsecp256k1::SecretKey;
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
pub struct L1TxSenderConfig {
    pub block_executor_address: Address,
    pub operator_address: Address,
    #[serde(deserialize_with = "secret_key_deserializer")]
    pub operator_private_key: SecretKey,
}

impl L1TxSenderConfig {
    pub fn from_env() -> Result<Self, String> {
        match envy::prefixed("L1_TX_SENDER_").from_env::<Self>() {
            Ok(config) => Ok(config),
            Err(error) => Err(error.to_string()),
        }
    }
}

fn secret_key_deserializer<'de, D>(deserializer: D) -> Result<SecretKey, D::Error>
where
    D: Deserializer<'de>,
{
    let hex = H256::deserialize(deserializer)?;
    SecretKey::parse(hex.as_fixed_bytes()).map_err(serde::de::Error::custom)
}
