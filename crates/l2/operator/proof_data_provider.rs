use std::{
    io::{BufReader, BufWriter},
    net::{IpAddr, TcpListener, TcpStream},
};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use sp1_sdk::SP1ProofWithPublicValues;
use tracing::{debug, info};

use crate::rpc::l1_rpc::RpcResponse;

pub async fn start_proof_data_provider(ip: IpAddr, port: u16) {
    let proof_data_provider = ProofDataProvider::new(ip, port);
    proof_data_provider.start().await;
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ProofData {
    Request {},
    Response {
        id: u64,
    },
    Submit {
        id: u64,
        proof: Box<SP1ProofWithPublicValues>,
    },
    SubmitAck {
        id: u64,
    },
}

struct ProofDataProvider {
    ip: IpAddr,
    port: u16,
}

impl ProofDataProvider {
    pub fn new(ip: IpAddr, port: u16) -> Self {
        Self { ip, port }
    }

    pub async fn start(&self) {
        let listener = TcpListener::bind(format!("{}:{}", self.ip, self.port)).unwrap();

        let mut last_proved_block = 0;

        info!("Starting TCP server at {}:{}", self.ip, self.port);
        for stream in listener.incoming() {
            let stream = stream.unwrap();

            debug!("Connection established!");
            self.handle_connection(stream, &mut last_proved_block).await;
        }
    }

    async fn handle_connection(&self, mut stream: TcpStream, last_proved_block: &mut u64) {
        let buf_reader = BufReader::new(&stream);

        let data: ProofData = serde_json::de::from_reader(buf_reader).unwrap();
        match data {
            ProofData::Request {} => self.handle_request(&mut stream, *last_proved_block).await,
            ProofData::Submit { id, proof } => {
                self.handle_submit(&mut stream, id, proof);
                *last_proved_block += 1;
            }
            _ => {}
        }

        debug!("Connection closed");
    }

    async fn get_last_block_number() -> Result<u64, String> {
        let response = Client::new()
            .post("http://localhost:8551")
            .header("content-type", "application/json")
            .body(
                r#"{
                    "jsonrpc": "2.0",
                    "method": "eth_blockNumber",
                    "params": [],
                    "id": 1
                }"#,
            )
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<RpcResponse>()
            .await
            .map_err(|e| e.to_string())?;

        if let RpcResponse::Success(r) = response {
            Ok(
                u64::from_str_radix(r.result.as_str().unwrap().strip_prefix("0x").unwrap(), 16)
                    .unwrap(),
            )
        } else {
            Err("Failed to get last block number".to_string())
        }
    }

    async fn handle_request(&self, stream: &mut TcpStream, last_proved_block: u64) {
        debug!("Request received");

        if let Ok(last_block_number) = Self::get_last_block_number().await {
            if last_block_number > last_proved_block {
                let response = ProofData::Response {
                    id: last_proved_block + 1,
                };
                let writer = BufWriter::new(stream);
                serde_json::to_writer(writer, &response).unwrap();
            }
        }
    }

    fn handle_submit(&self, stream: &mut TcpStream, id: u64, proof: Box<SP1ProofWithPublicValues>) {
        debug!("Submit received. ID: {id}, proof: {:?}", proof.proof);

        let response = ProofData::SubmitAck { id };
        let writer = BufWriter::new(stream);
        serde_json::to_writer(writer, &response).unwrap();
    }
}
