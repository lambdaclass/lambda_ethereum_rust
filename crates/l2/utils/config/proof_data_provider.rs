use std::net::IpAddr;

use serde::Deserialize;

use super::errors::ConfigError;

#[derive(Clone, Deserialize)]
pub struct ProofDataProviderConfig {
    pub listen_ip: IpAddr,
    pub listen_port: u16,
}

impl ProofDataProviderConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        envy::prefixed("PROOF_DATA_PROVIDER_")
            .from_env::<Self>()
            .map_err(ConfigError::from)
    }
}
