use std::{
    io::{BufReader, BufWriter},
    net::{IpAddr, TcpStream},
    time::Duration,
};

use sp1_sdk::SP1ProofWithPublicValues;
use tokio::time::sleep;
use tracing::{debug, warn};

use crate::operator::proof_data_provider::ProofData;

use super::sp1_prover::SP1Prover;

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
        let prover = SP1Prover::new();

        loop {
            let id = self.request_new_data().unwrap();

            let proof = prover.prove(id).unwrap();

            self.submit_proof(id, proof).unwrap();

            sleep(Duration::from_secs(5)).await;
        }
    }

    fn request_new_data(&self) -> Result<u32, String> {
        let stream = TcpStream::connect(format!("{}:{}", self.ip, self.port)).unwrap();
        let buf_writer = BufWriter::new(&stream);

        debug!("Connection established!");

        let request = ProofData::Request {};
        serde_json::ser::to_writer(buf_writer, &request).unwrap();
        stream.shutdown(std::net::Shutdown::Write).unwrap();

        let buf_reader = BufReader::new(&stream);
        let response: ProofData = serde_json::de::from_reader(buf_reader).unwrap();

        match response {
            ProofData::Response { id } => {
                debug!("Received response: {}", id);
                Ok(id)
            }
            _ => {
                warn!("Unexpected response: {:?}", response);
                Err("Unexpected response".to_string())
            }
        }
    }

    fn submit_proof(&self, id: u32, proof: SP1ProofWithPublicValues) -> Result<(), String> {
        let stream = TcpStream::connect(format!("{}:{}", self.ip, self.port)).unwrap();
        let buf_writer = BufWriter::new(&stream);

        let submit = ProofData::Submit {
            id,
            proof: Box::new(proof),
        };
        serde_json::ser::to_writer(buf_writer, &submit).unwrap();
        stream.shutdown(std::net::Shutdown::Write).unwrap();

        let buf_reader = BufReader::new(&stream);
        let response: ProofData = serde_json::de::from_reader(buf_reader).unwrap();
        match response {
            ProofData::SubmitAck { id: res_id } => {
                debug!("Received submit ack: {}", res_id);
                Ok(())
            }
            _ => {
                warn!("Unexpected response: {:?}", response);
                Err("Unexpected response".to_string())
            }
        }
    }
}
