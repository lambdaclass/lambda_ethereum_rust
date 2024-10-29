use std::{
    io::{BufReader, BufWriter},
    net::TcpStream,
    time::Duration,
};

use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use ethereum_rust_l2::{
    proposer::prover_server::{ProofData, ProverInputData},
    utils::config::prover_client::ProverClientConfig,
};

use super::prover::Prover;

pub async fn start_proof_data_client(config: ProverClientConfig) {
    let mut proof_data_client = ProverClient::new(config.prover_server_endpoint.clone());
    proof_data_client.start().await;
}

struct ProverClient {
    prover_server_endpoint: String,
    block_number_to_request: u64,
}

impl ProverClient {
    pub fn new(prover_server_endpoint: String) -> Self {
        let block_number_to_request: u64 = 1;
        Self {
            prover_server_endpoint,
            block_number_to_request,
        }
    }

    pub async fn start(&mut self) {
        let mut prover = Prover::new();

        loop {
            match self.request_new_input() {
                Ok((block_number, input)) => {
                    match prover.set_input(input).prove() {
                        Ok(proof) => {
                            if let Err(e) = self.submit_proof(block_number, proof) {
                                // TODO: Retry?
                                warn!("Failed to submit proof: {e}");
                            }
                        }
                        Err(e) => error!(e),
                    };
                }
                Err(e) => {
                    sleep(Duration::from_secs(10)).await;
                    warn!("Failed to request new data: {e}");
                }
            }
        }
    }

    fn request_new_input(&mut self) -> Result<(u64, ProverInputData), String> {
        warn!("endpoint {}", &self.prover_server_endpoint);

        let stream = TcpStream::connect(&self.prover_server_endpoint)
            .map_err(|e| format!("Failed to connect to Prover Server: {e}"))?;
        let buf_writer = BufWriter::new(&stream);

        debug!("Connection established!");

        let request = ProofData::Request {
            block_number: self.block_number_to_request,
        };
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
            } => {
                self.block_number_to_request = block_number;
                Ok((block_number, input))
            }
            _ => Err(format!("Unexpected response {response:?}")),
        }
    }

    fn submit_proof(
        &mut self,
        block_number: u64,
        receipt: risc0_zkvm::Receipt,
    ) -> Result<(), String> {
        let stream = TcpStream::connect(&self.prover_server_endpoint)
            .map_err(|e| format!("Failed to connect to Prover Server: {e}"))?;
        let buf_writer = BufWriter::new(&stream);

        let submit = ProofData::Submit {
            block_number,
            receipt: Box::new(receipt),
        };
        serde_json::ser::to_writer(buf_writer, &submit).map_err(|e| e.to_string())?;
        stream
            .shutdown(std::net::Shutdown::Write)
            .map_err(|e| e.to_string())?;

        let buf_reader = BufReader::new(&stream);
        let response: ProofData = serde_json::de::from_reader(buf_reader)
            .map_err(|e| format!("Invalid response format: {e}"))?;
        match response {
            ProofData::SubmitAck { block_number } => {
                info!("Received submit ack for block_number: {block_number}");
                // After submission, add 1 so that in the next request, the prover_client receives the subsequent block.
                self.block_number_to_request += 1;
                Ok(())
            }
            _ => Err(format!("Unexpected response {response:?}")),
        }
    }
}
