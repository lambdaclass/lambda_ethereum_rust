use serde::Deserialize;

use super::errors::ConfigError;

#[derive(Deserialize)]
pub struct ProverConfig {
    pub elf_path: String,
    pub proof_data_provider_endpoint: String,
}

impl ProverConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        envy::prefixed("PROVER_")
            .from_env::<Self>()
            .map_err(ConfigError::from)
    }
}
