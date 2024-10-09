use serde::Deserialize;

#[derive(Deserialize)]
pub struct EthConfig {
    pub rpc_url: String,
}

impl EthConfig {
    pub fn from_env() -> Result<Self, String> {
        match envy::prefixed("ETH_").from_env::<Self>() {
            Ok(config) => Ok(config),
            Err(error) => Err(error.to_string()),
        }
    }
}
