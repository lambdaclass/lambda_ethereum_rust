use tracing::info;

pub mod proof_data_client;
pub mod prover;

pub async fn start_prover() {
    let proof_data_client = tokio::spawn(proof_data_client::start_proof_data_client());

    tokio::try_join!(proof_data_client).unwrap();
    info!("Prover finished!");
}
