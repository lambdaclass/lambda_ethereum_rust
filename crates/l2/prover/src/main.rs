pub mod client;
pub mod prover;

use tracing::info;

#[tokio::main]
async fn main() {
    let proof_data_client = tokio::spawn(client::start_proof_data_client());

    tokio::try_join!(proof_data_client).unwrap();
    info!("Prover finished!");
}
