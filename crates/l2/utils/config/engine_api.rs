use serde::Deserialize;

#[derive(Deserialize)]
pub struct EngineApiConfig {
    pub rpc_url: String,
    pub jwt_path: String,
}

impl EngineApiConfig {
    pub fn from_env() -> Result<Self, String> {
        match envy::prefixed("ENGINE_API_").from_env::<Self>() {
            Ok(config) => Ok(config),
            Err(error) => Err(error.to_string()),
        }
    }
}
