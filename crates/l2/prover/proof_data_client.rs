#![allow(dead_code)]
#![allow(unused_imports)]

use std::{
    io::{BufReader, BufWriter, Read},
    net::{IpAddr, TcpStream},
    time::Duration,
};

use sp1_sdk::SP1ProofWithPublicValues;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use prover_lib::inputs::{ProverInput, ProverInputNoExecution};

use crate::operator::proof_data_provider::ProofData;

use super::zk_prover::Prover;

pub async fn start_proof_data_client(ip: IpAddr, port: u16) {
    let proof_data_client = ProofDataClient::new(ip, port);
    proof_data_client.start().await;
}

struct ProofDataClient {
    ip: IpAddr,
    port: u16,
}

impl ProofDataClient {
    pub fn new(ip: IpAddr, port: u16) -> Self {
        Self { ip, port }
    }

    pub async fn start(&self) {
        let prover_verification = Prover::new_verification();

        loop {
            match self.request_new_verification_data() {
                Ok((Some(id), prover_input)) => {
                    match prover_verification.prove(Box::new(prover_input)) {
                        Ok(proof) => {
                            if let Err(e) = self.submit_proof(id, proof) {
                                // TODO: Retry
                                warn!("Failed to submit proof: {e}");
                            }
                        }
                        Err(e) => error!(e),
                    };
                }
                Ok(_) => sleep(Duration::from_secs(10)).await,
                Err(e) => {
                    warn!("Failed to request new data: {e}");
                    sleep(Duration::from_secs(10)).await;
                }
            }
            info!("The Prover has finished the last round.");
        }
    }

    fn request_new_verification_data(
        &self,
    ) -> Result<(Option<u64>, ProverInputNoExecution), String> {
        let stream = TcpStream::connect(format!("{}:{}", self.ip, self.port))
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
                id: Some(id),
                prover_inputs: Some(prover_inputs),
            } => {
                debug!("Received response ID: {id:?}");
                debug!("Received response block: {:?}", prover_inputs.head_block);

                Ok((Some(id), prover_inputs))
            }
            _ => Err(format!("Unexpected response {response:?}")),
        }
    }

    fn submit_proof(&self, id: u64, proof: SP1ProofWithPublicValues) -> Result<(), String> {
        let stream = TcpStream::connect(format!("{}:{}", self.ip, self.port))
            .map_err(|e| format!("Failed to connect to ProofDataProvider: {e}"))?;
        let buf_writer = BufWriter::new(&stream);

        let submit = ProofData::Submit {
            id,
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
            ProofData::SubmitAck { id: res_id } => {
                debug!("Received submit ack: {res_id}");
                Ok(())
            }
            _ => Err(format!("Unexpected response {response:?}")),
        }
    }
}
