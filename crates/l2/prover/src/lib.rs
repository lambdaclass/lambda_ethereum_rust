pub mod client;
pub mod prover;

use ethereum_rust_l2::utils::config::prover::ProverConfig;
use tracing::warn;

pub async fn init_client(config: ProverConfig) {
    client::start_proof_data_client(config).await;
    warn!("Prover finished!");
}
