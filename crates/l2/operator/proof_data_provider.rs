use std::{
    io::{BufReader, BufWriter},
    net::{IpAddr, TcpListener, TcpStream},
};

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

pub async fn start_proof_data_provider(ip: IpAddr, port: u16) {
    let proof_data_provider = ProofDataProvider::new(ip, port);
    proof_data_provider.start();
}

#[derive(Debug, Serialize, Deserialize)]
enum ProofData {
    Request {},
    Response { id: u32 },
    Submit { id: u32 },
    SubmitAck { id: u32 },
}

struct ProofDataProvider {
    ip: IpAddr,
    port: u16,
}

impl ProofDataProvider {
    pub fn new(ip: IpAddr, port: u16) -> Self {
        Self { ip, port }
    }

    pub fn start(&self) {
        let listener = TcpListener::bind(format!("{}:{}", self.ip, self.port)).unwrap();

        info!("Starting TCP server at {}:{}", self.ip, self.port);
        for stream in listener.incoming() {
            let stream = stream.unwrap();

            debug!("Connection established!");
            self.handle_connection(stream);
        }
    }

    fn handle_connection(&self, mut stream: TcpStream) {
        let buf_reader = BufReader::new(&stream);

        let data: ProofData = serde_json::de::from_reader(buf_reader).unwrap();
        debug!("ProofData: {:?}", data);
        match data {
            ProofData::Request {} => self.handle_request(&mut stream),
            ProofData::Submit { id } => self.handle_submit(&mut stream, id),
            _ => {}
        }

        debug!("Connection closed");
    }

    fn handle_request(&self, stream: &mut TcpStream) {
        debug!("Request received");

        let response = ProofData::Response { id: 1 };
        let writer = BufWriter::new(stream);
        serde_json::to_writer(writer, &response).unwrap();
    }

    fn handle_submit(&self, stream: &mut TcpStream, id: u32) {
        debug!("Submit received");

        let response = ProofData::SubmitAck { id };
        let writer = BufWriter::new(stream);
        serde_json::to_writer(writer, &response).unwrap();
    }
}
