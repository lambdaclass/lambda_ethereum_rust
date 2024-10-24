use ethereum_rust_l2::utils::config::{prover_client::ProverClientConfig, read_env_file};
use ethereum_rust_prover_lib::init_client;

use tracing::{self, debug, level_filters::LevelFilter, warn};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .init();

    if let Err(e) = read_env_file() {
        warn!("Failed to read .env file: {e}");
    }

    let config = ProverClientConfig::from_env().unwrap();
    debug!("Prover Client has started");
    init_client(config).await;
}
