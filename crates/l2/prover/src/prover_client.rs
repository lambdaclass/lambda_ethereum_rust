use std::{
    io::{BufReader, BufWriter},
    net::TcpStream,
    time::Duration,
};

use sp1_sdk::SP1ProofWithPublicValues;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use ethereum_rust_l2::{
    proposer::prover_server::{ProofData, ProverInputData},
    utils::config::prover_client::ProverClientConfig,
};

use super::prover::Prover;

pub async fn start_proof_data_client(config: ProverClientConfig) {
    let proof_data_client = ProofDataClient::new(config.prover_server_endpoint.clone());
    proof_data_client.start(config).await;
}

struct ProofDataClient {
    prover_server_endpoint: String,
}

impl ProofDataClient {
    pub fn new(prover_server_endpoint: String) -> Self {
        Self {
            prover_server_endpoint,
        }
    }

    pub async fn start(&self, config: ProverClientConfig) {
        let mut prover = Prover::new_from_config(config);

        loop {
            match self.request_new_data() {
                Ok((Some(block_number), input)) => {
                    match prover.set_input(input).prove() {
                        Ok(proof) => {
                            if let Err(e) = self.submit_proof(block_number, proof) {
                                // TODO: Retry
                                warn!("Failed to submit proof: {e}");
                            }
                        }
                        Err(e) => error!(e),
                    };
                }
                Ok((None, _)) => sleep(Duration::from_secs(10)).await,
                Err(e) => warn!("Failed to request new data: {e}"),
            }
        }
    }

    fn request_new_data(&self) -> Result<(Option<u64>, ProverInputData), String> {
        let stream = TcpStream::connect(&self.prover_server_endpoint)
            .map_err(|e| format!("Failed to connect to ProofDataProvider: {e}"))?;
        let buf_writer = BufWriter::new(&stream);

        debug!("Connection established!");

        let request = ProofData::Request {};
        serde_json::ser::to_writer(buf_writer, &request).map_err(|e| e.to_string())?;
        stream
            .shutdown(std::net::Shutdown::Write)
            .map_err(|e| e.to_string())?;

        let buf_reader = BufReader::new(&stream);
        let response: ProofData = serde_json::de::from_reader(buf_reader)
            .map_err(|e| format!("Invalid response format: {e}"))?;

        match response {
            ProofData::Response {
                block_number,
                input,
            } => Ok((block_number, input)),
            _ => Err(format!("Unexpected response {response:?}")),
        }
    }

    fn submit_proof(
        &self,
        block_number: u64,
        proof: SP1ProofWithPublicValues,
    ) -> Result<(), String> {
        let stream = TcpStream::connect(&self.prover_server_endpoint)
            .map_err(|e| format!("Failed to connect to ProofDataProvider: {e}"))?;
        let buf_writer = BufWriter::new(&stream);

        let submit = ProofData::Submit {
            block_number,
            proof: Box::new(proof),
        };
        serde_json::ser::to_writer(buf_writer, &submit).map_err(|e| e.to_string())?;
        stream
            .shutdown(std::net::Shutdown::Write)
            .map_err(|e| e.to_string())?;

        let buf_reader = BufReader::new(&stream);
        let response: ProofData = serde_json::de::from_reader(buf_reader)
            .map_err(|e| format!("Invalid response format: {e}"))?;
        match response {
            ProofData::SubmitAck {
                block_number: res_id,
            } => {
                info!("Received submit ack: {res_id}");
                Ok(())
            }
            _ => Err(format!("Unexpected response {response:?}")),
        }
    }
}
