use serde::Deserialize;

#[derive(Deserialize)]
pub struct ProverConfig {
    pub elf_path: String,
    pub proof_data_provider_endpoint: String,
}

impl ProverConfig {
    pub fn from_env() -> Result<Self, String> {
        match envy::prefixed("PROVER_").from_env::<Self>() {
            Ok(config) => Ok(config),
            Err(error) => Err(error.to_string()),
        }
    }
}
