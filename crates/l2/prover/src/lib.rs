pub mod prover;
pub mod prover_client;
pub mod utils;

use ethereum_rust_l2::utils::config::prover_client::ProverClientConfig;
use tracing::warn;

pub async fn init_client(config: ProverClientConfig) {
    // TODO: panicking if the client fails. Improve error handling
    prover_client::start_proof_data_client(config)
        .await
        .unwrap();
    warn!("Prover finished!");
}
