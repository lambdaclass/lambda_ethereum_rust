use std::net::IpAddr;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct ProofDataProviderConfig {
    pub listen_ip: IpAddr,
    pub listen_port: u16,
}

impl ProofDataProviderConfig {
    pub fn from_env() -> Result<Self, String> {
        match envy::prefixed("PROOF_DATA_PROVIDER_").from_env::<Self>() {
            Ok(config) => Ok(config),
            Err(error) => Err(error.to_string()),
        }
    }
}
