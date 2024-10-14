use std::io::BufRead;

use tracing::debug;

pub mod engine_api;
pub mod eth;
pub mod l1_watcher;
pub mod operator;
pub mod proof_data_provider;
pub mod prover;

pub mod errors;

pub fn read_env_file() -> Result<(), errors::ConfigError> {
    let env_file = std::fs::File::open(".env")?;
    let reader = std::io::BufReader::new(env_file);

    for line in reader.lines() {
        match line?.split_once('=') {
            Some((key, value)) => {
                debug!("Setting env var from .env: {key}={value}");
                std::env::set_var(key, value)
            }
            None => continue,
        };
    }

    Ok(())
}
