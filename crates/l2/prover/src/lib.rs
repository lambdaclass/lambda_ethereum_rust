pub mod client;
pub mod prover;

use tracing::info;

pub async fn init_client() {
    let client = tokio::spawn(client::start_proof_data_client());

    tokio::try_join!(client).unwrap();
    info!("Prover finished!");
}
