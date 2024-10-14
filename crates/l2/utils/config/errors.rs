#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Error deserializing config from env: {0}")]
    ConfigDeserializationError(#[from] envy::Error),
    #[error("Error reading env file: {0}")]
    EnvFileError(#[from] std::io::Error),
}
