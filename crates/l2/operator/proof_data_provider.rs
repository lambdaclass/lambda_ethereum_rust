use std::{
    io::{BufReader, BufWriter},
    net::{IpAddr, TcpListener, TcpStream},
};

use serde::{Deserialize, Serialize};
use sp1_sdk::SP1ProofWithPublicValues;
use tracing::{debug, info};

use crate::utils::config::proof_data_provider::ProofDataProviderConfig;

pub async fn start_proof_data_provider() {
    let config = ProofDataProviderConfig::from_env().unwrap();
    let proof_data_provider = ProofDataProvider::new_from_config(config);
    proof_data_provider.start();
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ProofData {
    Request {},
    Response {
        id: u32,
    },
    Submit {
        id: u32,
        proof: Box<SP1ProofWithPublicValues>,
    },
    SubmitAck {
        id: u32,
    },
}

struct ProofDataProvider {
    ip: IpAddr,
    port: u16,
}

impl ProofDataProvider {
    pub fn new_from_config(config: ProofDataProviderConfig) -> Self {
        Self {
            ip: config.listen_ip,
            port: config.listen_port,
        }
    }

    pub fn start(&self) {
        let listener = TcpListener::bind(format!("{}:{}", self.ip, self.port)).unwrap();

        let mut current_id = 1;

        info!("Starting TCP server at {}:{}", self.ip, self.port);
        for stream in listener.incoming() {
            let stream = stream.unwrap();

            debug!("Connection established!");
            self.handle_connection(stream, &mut current_id);
        }
    }

    fn handle_connection(&self, mut stream: TcpStream, current_id: &mut u32) {
        let buf_reader = BufReader::new(&stream);

        let data: ProofData = serde_json::de::from_reader(buf_reader).unwrap();
        match data {
            ProofData::Request {} => {
                self.handle_request(&mut stream, *current_id);
                *current_id = (*current_id % 20) + 1;
            }
            ProofData::Submit { id, proof } => self.handle_submit(&mut stream, id, proof),
            _ => {}
        }

        debug!("Connection closed");
    }

    fn handle_request(&self, stream: &mut TcpStream, current_id: u32) {
        debug!("Request received");

        let response = ProofData::Response { id: current_id };
        let writer = BufWriter::new(stream);
        serde_json::to_writer(writer, &response).unwrap();
    }

    fn handle_submit(&self, stream: &mut TcpStream, id: u32, proof: Box<SP1ProofWithPublicValues>) {
        debug!("Submit received. ID: {id}, proof: {:?}", proof.proof);

        let response = ProofData::SubmitAck { id };
        let writer = BufWriter::new(stream);
        serde_json::to_writer(writer, &response).unwrap();
    }
}
