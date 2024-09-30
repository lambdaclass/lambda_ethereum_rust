use std::net::{IpAddr, Ipv4Addr};

use tracing::info;

pub mod proof_data_client;
pub mod sp1_prover;

pub async fn start_prover() {
    let proof_data_client = tokio::spawn(proof_data_client::start_proof_data_client(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        3000,
    ));

    tokio::try_join!(proof_data_client).unwrap();
    info!("Prover finished!");
}
