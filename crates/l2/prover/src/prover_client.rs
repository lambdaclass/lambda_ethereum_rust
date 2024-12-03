use std::{
    io::{BufReader, BufWriter},
    net::TcpStream,
    time::Duration,
};

use sp1_sdk::network::prover;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use zkvm_interface::io::ProgramInput;

use ethrex_l2::{
    proposer::prover_server::{ProofData, ProverType, ZkProof},
    utils::config::prover_client::ProverClientConfig,
};

use crate::prover::{Prover, ProvingOutput, Risc0Prover};

pub async fn start_proof_data_client(config: ProverClientConfig) {
    let proof_data_client = ProverClient::new(config);
    proof_data_client.start().await;
}

struct ProverData {
    block_number: u64,
    input: ProgramInput,
    prover_type: ProverType
}

struct ProverClient {
    prover_server_endpoint: String,
    interval_ms: u64,
}

impl ProverClient {
    pub fn new(config: ProverClientConfig) -> Self {
        Self {
            prover_server_endpoint: config.prover_server_endpoint,
            interval_ms: config.interval_ms,
        }
    }

    pub async fn start(&self) {
        loop {
            match self.request_new_input() {
                // If we get the input
                Ok(prover_data) => {
                    // Check the prover_type requested by the ProverServer
                    match prover_data.prover_type {
                        ProverType::RISC0 => {
                            let mut prover = Risc0Prover::new();
                            match prover.prove(prover_data.input) {
                                Ok(proof) => {
                                    if let Err(e) =
                                        self.submit_proof(prover_data.block_number, proof, prover.id.to_vec())
                                    {
                                        // TODO: Retry?
                                        warn!("Failed to submit proof: {e}");
                                    }
                                }
                                Err(e) => error!(e),
                            };
                        },
                        ProverType::SP1 => todo!(),
                    }

                }
                Err(e) => {
                    sleep(Duration::from_millis(self.interval_ms)).await;
                    warn!("Failed to request new data: {e}");
                }
            }
        }
    }

    fn request_new_input(&self) -> Result<ProverData, String> {
        // Request the input with the correct block_number
        let request = ProofData::Request;
        let response = connect_to_prover_server_wr(&self.prover_server_endpoint, &request)
            .map_err(|e| format!("Failed to get Response: {e}"))?;

        match response {
            ProofData::Response {
                block_number,
                input,
                prover_type
                
            } => match (block_number, input) {
                (Some(block_number), Some(input)) => {
                    info!("Received Response for block_number: {block_number}");
                    let prover_data = ProverData{
                        block_number,
                        input:  ProgramInput {
                            block: input.block,
                            parent_block_header: input.parent_block_header,
                            db: input.db
                        },
                        prover_type,
                    };
                    Ok(prover_data)
                }
                _ => Err(
                    "Received Empty Response, meaning that the ProverServer doesn't have blocks to prove.\nThe Prover may be advancing faster than the Proposer."
                        .to_owned(),
                ),
            },
            _ => Err(format!("Expecting ProofData::Response  {response:?}")),
        }
    }

    fn submit_proof(
        &self,
        block_number: u64,
        proving_output: ProvingOutput,
        prover_id: Vec<u32>,
    ) -> Result<(), String> {
        // TODO replace
        let receipt = match proving_output {
            ProvingOutput::Risc0Prover(receipt) => receipt,
            ProvingOutput::Sp1Prover(_) => todo!(),
        };
        let submit = ProofData::Submit {
            block_number,
            zk_proof: ZkProof::RISC0(Box::new((*receipt, prover_id))),
        };
        let submit_ack = connect_to_prover_server_wr(&self.prover_server_endpoint, &submit)
            .map_err(|e| format!("Failed to get SubmitAck: {e}"))?;

        match submit_ack {
            ProofData::SubmitAck { block_number } => {
                info!("Received submit ack for block_number: {}", block_number);
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
