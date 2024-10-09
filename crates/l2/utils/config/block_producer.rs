use serde::Deserialize;

#[derive(Deserialize)]
pub struct BlockProducerConfig {
    pub interval_ms: u64,
}

impl BlockProducerConfig {
    pub fn from_env() -> Result<Self, String> {
        match envy::prefixed("BLOCK_PRODUCER_").from_env::<Self>() {
            Ok(config) => Ok(config),
            Err(error) => Err(error.to_string()),
        }
    }
}
