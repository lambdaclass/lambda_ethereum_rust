#![allow(dead_code)]
#![allow(unused_imports)]

use std::{
    io::{BufReader, BufWriter},
    net::{IpAddr, TcpListener, TcpStream},
    str::FromStr,
};

use ethereum_rust_core::types::{Block, BlockBody, BlockHeader};
use ethereum_rust_storage::Store;
use ethereum_types::{Bloom, H160, H256, U256};
use prover_lib::{
    db_memorydb::MemoryDB,
    inputs::{read_chain_file, ProverInput, ProverInputNoExecution},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sp1_sdk::{network::prover, SP1ProofWithPublicValues};
use tracing::{debug, info, warn};

use crate::{
    prover::zk_prover::{Prover, ProverMode},
    rpc::l1_rpc::RpcResponse,
};

use revm::{db::CacheDB, InMemoryDB};

pub async fn start_proof_data_provider(store: Store, ip: IpAddr, port: u16) {
    let proof_data_provider = ProofDataProvider::new(store, ip, port);
    proof_data_provider.start().await;
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ProofData {
    Request {
        mode: ProverMode,
    },
    Response {
        id: Option<u64>,
        prover_inputs_verification: Option<ProverInputNoExecution>,
        prover_inputs_execution: Option<ProverInput>,
        mode: ProverMode,
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
    store: Store,
    ip: IpAddr,
    port: u16,
}

impl ProofDataProvider {
    pub fn new(store: Store, ip: IpAddr, port: u16) -> Self {
        Self { store, ip, port }
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

        let data: Result<ProofData, _> = serde_json::de::from_reader(buf_reader);
        match data {
            Ok(ProofData::Request { mode }) => {
                info!("HANDLING proof_data_client REQUEST");
                if let Err(e) = self
                    .handle_request(&mut stream, mode, *last_proved_block)
                    .await
                {
                    warn!("Failed to handle request: {e}");
                }
            }
            Ok(ProofData::Submit { id, proof }) => {
                if let Err(e) = self.handle_submit(&mut stream, id, proof) {
                    warn!("Failed to handle submit: {e}");
                }
                *last_proved_block += 1;
            }
            Err(e) => {
                warn!("Failed to parse request: {e}");
            }
            _ => {
                warn!("Invalid request");
            }
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
            u64::from_str_radix(
                r.result
                    .as_str()
                    .ok_or("Response format error".to_string())?
                    .strip_prefix("0x")
                    .ok_or("Response format error".to_string())?,
                16,
            )
            .map_err(|e| e.to_string())
        } else {
            Err("Failed to get last block number".to_string())
        }
    }

    fn convert_value_to_block(value: &serde_json::Value) -> Result<Block, String> {
        // Check if the value is an object
        if let serde_json::Value::Object(obj) = value {
            let header = BlockHeader {
                parent_hash: {
                    let hash = obj
                        .get("parentHash")
                        .and_then(serde_json::Value::as_str)
                        .ok_or("Missing parentHash")?
                        .to_string();
                    H256::from_str(&hash).map_err(|e| e.to_string())?
                },
                number: u64::from_str_radix(
                    obj.get("number")
                        .and_then(serde_json::Value::as_str)
                        .ok_or("Missing number")?
                        .strip_prefix("0x")
                        .ok_or("Missing number")?,
                    16,
                )
                .map_err(|e| e.to_string())?,
                ommers_hash: {
                    let hash = obj
                        .get("sha3Uncles")
                        .and_then(serde_json::Value::as_str)
                        .ok_or("Missing sha3Uncles")?
                        .to_string();
                    H256::from_str(&hash).map_err(|e| e.to_string())?
                },
                coinbase: H160::default(),
                state_root: {
                    let hash = obj
                        .get("stateRoot")
                        .and_then(serde_json::Value::as_str)
                        .ok_or("Missing stateRoot")?
                        .to_string();
                    H256::from_str(&hash).map_err(|e| e.to_string())?
                },
                transactions_root: {
                    let hash = obj
                        .get("transactionsRoot")
                        .and_then(serde_json::Value::as_str)
                        .ok_or("Missing transactionsRoot")?
                        .to_string();
                    H256::from_str(&hash).map_err(|e| e.to_string())?
                },
                receipts_root: {
                    let hash = obj
                        .get("receiptsRoot")
                        .and_then(serde_json::Value::as_str)
                        .ok_or("Missing receiptRoot")?
                        .to_string();
                    H256::from_str(&hash).map_err(|e| e.to_string())?
                },
                // TODO
                logs_bloom: Bloom::default(),
                difficulty: U256::from_str_radix(
                    obj.get("difficulty")
                        .and_then(serde_json::Value::as_str)
                        .ok_or("Missing difficulty")?
                        .strip_prefix("0x")
                        .ok_or("Missing difficulty")?,
                    16,
                )
                .map_err(|e| e.to_string())?,
                gas_limit: u64::from_str_radix(
                    obj.get("gasLimit")
                        .and_then(serde_json::Value::as_str)
                        .ok_or("Missing gasLimit")?
                        .strip_prefix("0x")
                        .ok_or("Missing gasLimit")?,
                    16,
                )
                .map_err(|e| e.to_string())?,
                gas_used: u64::from_str_radix(
                    obj.get("gasUsed")
                        .and_then(serde_json::Value::as_str)
                        .ok_or("Missing gasUsed")?
                        .strip_prefix("0x")
                        .ok_or("Missing gasUsed")?,
                    16,
                )
                .map_err(|e| e.to_string())?,
                timestamp: u64::from_str_radix(
                    obj.get("timestamp")
                        .and_then(serde_json::Value::as_str)
                        .ok_or("Missing timestamp")?
                        .strip_prefix("0x")
                        .ok_or("Missing timestamp")?,
                    16,
                )
                .map_err(|e| e.to_string())?,
                extra_data: {
                    let extra_data = obj
                        .get("extraData")
                        .and_then(serde_json::Value::as_str)
                        .ok_or("Missing extraData")?
                        .strip_prefix("0x")
                        .ok_or("Missing extraData")?;
                    bytes::Bytes::copy_from_slice(extra_data.as_bytes())
                },
                // TODO
                prev_randao: H256::default(),
                nonce: u64::from_str_radix(
                    obj.get("nonce")
                        .and_then(serde_json::Value::as_str)
                        .ok_or("Missing nonce")?
                        .strip_prefix("0x")
                        .ok_or("Nonce error")?,
                    16,
                )
                .map_err(|e| e.to_string())?,
                base_fee_per_gas: Some(
                    u64::from_str_radix(
                        obj.get("gasLimit")
                            .and_then(serde_json::Value::as_str)
                            .ok_or("Missing gasLimit")?
                            .strip_prefix("0x")
                            .unwrap(),
                        16,
                    )
                    .map_err(|e| e.to_string())?,
                ),
                withdrawals_root: {
                    let hash = obj
                        .get("withdrawalsRoot")
                        .and_then(serde_json::Value::as_str)
                        .ok_or("Missing withdrawalsRoot")?
                        .to_string();
                    Some(H256::from_str(&hash).map_err(|e| e.to_string())?)
                },
                blob_gas_used: Some(
                    u64::from_str_radix(
                        obj.get("blobGasUsed")
                            .and_then(serde_json::Value::as_str)
                            .ok_or("Missing blobGasUsed")?
                            .strip_prefix("0x")
                            .unwrap(),
                        16,
                    )
                    .map_err(|e| e.to_string())?,
                ),
                excess_blob_gas: Some(
                    u64::from_str_radix(
                        obj.get("excessBlobGas")
                            .and_then(serde_json::Value::as_str)
                            .ok_or("Missing excessBlobGas")?
                            .strip_prefix("0x")
                            .unwrap(),
                        16,
                    )
                    .map_err(|e| e.to_string())?,
                ),
                parent_beacon_block_root: {
                    let hash = obj
                        .get("parentBeaconBlockRoot")
                        .and_then(serde_json::Value::as_str)
                        .ok_or("Missing parentBeaconBlockRoot")?
                        .to_string();
                    Some(H256::from_str(&hash).map_err(|e| e.to_string())?)
                },
            };

            // TODO the BlockBody should be parsed too
            let body = BlockBody {
                transactions: [].to_vec(),
                ommers: [].to_vec(),
                withdrawals: Some([].to_vec()),
            };

            Ok(Block { header, body })
        } else {
            Err("Expected a JSON object".to_string())
        }
    }

    async fn get_block_by_number(block_number: u64, full: bool) -> Result<Block, String> {
        let hex_string_block = format!("0x{:x}", block_number);
        let json_body = format!(
            r#"{{ 
                "jsonrpc": "2.0", 
                "method": "eth_getBlockByNumber", 
                "params": ["{}", {}], 
                "id": 1 
            }}"#,
            hex_string_block, full
        );

        let response = Client::new()
            .post("http://localhost:8551")
            .header("content-type", "application/json")
            .body(json_body)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<RpcResponse>()
            .await
            .map_err(|e| e.to_string())?;

        if let RpcResponse::Success(r) = response {
            // Attempt to deserialize the response into a Block struct
            match Self::convert_value_to_block(&r.result) {
                Ok(block) => Ok(block),
                Err(e) => Err(e),
            }
        } else {
            Err("Failed to get block by number".to_string())
        }
    }

    async fn handle_request(
        &self,
        stream: &mut TcpStream,
        prover_mode: ProverMode,
        last_proved_block: u64,
    ) -> Result<(), String> {
        debug!("Request received");

        let _last_block_number = Self::get_last_block_number().await?;
        //let block = Self::get_block_by_number(last_block_number, false).await?;
        //
        //let parent_block_header = {
        //    if last_block_number > 0 {
        //        let parent_block = Self::get_block_by_number(last_block_number - 1, false).await?;
        //        parent_block.header
        //    } else {
        //        BlockHeader::default()
        //    }
        //};

        // Crate Mismatch
        //let parent_block_header = self
        //    .store
        //    .get_block_header(last_block_number - 1)
        //    .unwrap()
        //    .unwrap();

        //let block = Block{header, body}
        //let state = MemoryDB::new(accounts, storage, block_hashes);

        //info!("get block by number result: {block:?}");
        //let state = MemoryDB::default();

        // Build Inputs
        let mut blocks = read_chain_file("./test_data/chain.rlp");
        let head_block = blocks.pop().unwrap();
        let parent_block_header = blocks.pop().unwrap().header;

        let prover_inputs_verification = ProverInputNoExecution {
            head_block: head_block.clone(),
            parent_block_header: parent_block_header.clone(),
            block_is_valid: false,
        };

        let prover_inputs_execution = ProverInput {
            block: head_block,
            parent_block_header,
            db: MemoryDB::default(),
        };

        let response = match prover_mode {
            ProverMode::Execution => {
                // This condition has to be true.
                //let response = if last_block_number > last_proved_block {
                if true {
                    ProofData::Response {
                        id: Some(last_proved_block + 1),
                        prover_inputs_verification: None,
                        prover_inputs_execution: Some(prover_inputs_execution),
                        mode: prover_mode,
                    }
                } else {
                    ProofData::Response {
                        id: None,
                        prover_inputs_verification: None,
                        prover_inputs_execution: None,
                        mode: prover_mode,
                    }
                }
            }
            ProverMode::Verification => {
                // This condition has to be true.
                //let response = if last_block_number > last_proved_block {
                if true {
                    ProofData::Response {
                        id: Some(last_proved_block + 1),
                        prover_inputs_verification: Some(prover_inputs_verification),
                        prover_inputs_execution: None,
                        mode: prover_mode,
                    }
                } else {
                    ProofData::Response {
                        id: None,
                        prover_inputs_verification: None,
                        prover_inputs_execution: None,
                        mode: prover_mode,
                    }
                }
            }
        };

        let writer = BufWriter::new(stream);
        serde_json::to_writer(writer, &response).map_err(|e| e.to_string())
    }

    fn handle_submit(
        &self,
        stream: &mut TcpStream,
        id: u64,
        proof: Box<SP1ProofWithPublicValues>,
    ) -> Result<(), String> {
        info!("Submit received. ID: {id}, proof: {:?}", proof.proof);

        let response = ProofData::SubmitAck { id };
        let writer = BufWriter::new(stream);
        serde_json::to_writer(writer, &response).map_err(|e| e.to_string())
    }
}
