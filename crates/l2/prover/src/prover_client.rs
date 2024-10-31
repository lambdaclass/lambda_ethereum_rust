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

use crate::utils::prover_state::{self, persist_block_in_prover_state, read_block_in_prover_state};

use super::prover::Prover;

pub async fn start_proof_data_client(config: ProverClientConfig) {
    let mut proof_data_client = ProverClient::new(config);
    proof_data_client.start().await;
}

struct ProverClient {
    prover_server_endpoint: String,
    prover_state_file_path: String,
    block_number_to_request: u64,
}

impl ProverClient {
    pub fn new(config: ProverClientConfig) -> Self {
        let prover_state_file_path = config
            .prover_state_file_path
            // TODO: rm unwrap
            .unwrap_or_else(|| prover_state::get_default_prover_state_file_path().unwrap());

        let block_number_to_request = match read_block_in_prover_state(&prover_state_file_path) {
            Ok(ps) => ps.block_header.number,
            Err(_) => 1,
        };
        Self {
            prover_server_endpoint: config.prover_server_endpoint,
            prover_state_file_path,
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
                    sleep(Duration::from_secs(5)).await;
                    warn!(
                        "Failed to request new data, block_number_to_request: {}. Error: {e}",
                        self.block_number_to_request
                    );
                }
            }
        }
    }

    fn request_new_input(&mut self) -> Result<(u64, ProverInputData), String> {
        // Request the input with the correct block_number
        let request = ProofData::Request {
            block_number: self.block_number_to_request,
        };
        let response = connect_to_prover_server_wr(&self.prover_server_endpoint, &request)
            .map_err(|e| format!("Failed to get Response: {e}"))?;

        match response {
            ProofData::Response {
                block_number,
                input,
            } => match (block_number, input) {
                (Some(n), Some(i)) => {
                    info!("Received Response for block_number: {n}");
                    self.block_number_to_request = n;
                    Ok((n, i))
                }
                _ => Err(
                    "Received Empty Response, meaning that the block requested isn't stored in the ProverServer"
                        .to_owned(),
                ),
            },
            _ => Err(format!("Expecting ProofData::Response  {response:?}")),
        }
    }

    fn submit_proof(
        &mut self,
        block_number: u64,
        receipt: risc0_zkvm::Receipt,
    ) -> Result<(), String> {
        let submit = ProofData::Submit {
            block_number,
            receipt: Box::new(receipt),
        };
        let submit_ack = connect_to_prover_server_wr(&self.prover_server_endpoint, &submit)
            .map_err(|e| format!("Failed to get SubmitAck: {e}"))?;

        match submit_ack {
            ProofData::SubmitAck { block_header } => {
                info!(
                    "Received submit ack for block_number: {}",
                    block_header.number
                );
                // After submission, add 1 so that in the next request, the prover_client receives the subsequent block.
                self.block_number_to_request += 1;
                // Persist the State
                persist_block_in_prover_state(&self.prover_state_file_path, block_header)
                    .map_err(|e| format!("Error while persisting state: {e}"))?;
                Ok(())
            }
            _ => Err(format!("Expecting ProofData::SubmitAck {submit_ack:?}")),
        }
    }
}

fn connect_to_prover_server_wr(
    addr: &str,
    write: &ProofData,
) -> Result<ProofData, Box<dyn std::error::Error>> {
    let stream = TcpStream::connect(addr)?;
    let buf_writer = BufWriter::new(&stream);
    debug!("Connection established!");
    serde_json::ser::to_writer(buf_writer, write)?;
    stream.shutdown(std::net::Shutdown::Write)?;

    let buf_reader = BufReader::new(&stream);
    let response: ProofData = serde_json::de::from_reader(buf_reader)?;
    Ok(response)
}
