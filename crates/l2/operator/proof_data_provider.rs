#![allow(dead_code)]
#![allow(unused_imports)]

use std::{
    collections::HashMap,
    io::{BufReader, BufWriter, Read},
    net::{IpAddr, TcpListener, TcpStream},
    os::macos::raw::stat,
    str::FromStr,
};

use ethereum_rust_blockchain::{
    find_parent_header, validate_block, validate_gas_used, validate_parent_canonical,
    validate_state_root,
};
use ethereum_rust_core::types::{Block, BlockBody, BlockHeader, TxKind};
use ethereum_rust_evm::{evm_state, execute_block, get_state_transitions, RevmAddress};
use ethereum_rust_storage::Store;
use ethereum_types::{Address, Bloom, H160, H256, U256};
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

use revm::{
    db::CacheDB,
    primitives::{bitvec::view::AsBits, AccountInfo, FixedBytes, B256},
    Evm, InMemoryDB,
};

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

        //let _last_block_number = Self::get_last_block_number().await?;

        //let state = MemoryDB::new(accounts, storage, block_hashes);

        // Build Inputs
        let mut blocks = read_chain_file("./test_data/chain.rlp");
        let head_block = blocks.pop().unwrap();
        let parent_block_header = blocks.pop().unwrap().header;

        let _prover_inputs_verification = ProverInputNoExecution {
            head_block: head_block.clone(),
            parent_block_header: parent_block_header.clone(),
            block_is_valid: false,
        };

        // we need the storage of the current_block-1
        // we should execute the EVM with that state and simulating the inclusion of the current_block
        let memory_db = get_last_block_state(&self.store).map_err(|e| format!("error code: {e}"));
        // with the execution_outputs we should get a way to have the State/Store represented with Hashmaps

        // finally, this information has to be contained in an structure that can be de/serealized,
        // so that, any zkVM can receive the state as input and prove the block execution.

        let prover_inputs_execution = ProverInput {
            block: head_block,
            parent_block_header,
            db: MemoryDB::default(),
        };

        let response = match prover_mode {
            // This condition has to be true.
            //if last_block_number > last_proved_block
            ProverMode::Execution if true => ProofData::Response {
                id: Some(last_proved_block + 1),
                prover_inputs_verification: None,
                prover_inputs_execution: Some(prover_inputs_execution),
                mode: prover_mode,
            },
            _ => ProofData::Response {
                id: None,
                prover_inputs_verification: None,
                prover_inputs_execution: None,
                mode: prover_mode,
            },
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

/// Same concept as adding a block [ethereum_rust_blockchain::add_block].
fn get_last_block_state(storage: &Store) -> Result<MemoryDB, Box<dyn std::error::Error>> {
    let last_block_number = storage.get_latest_block_number()?.unwrap();
    let body = storage.get_block_body(last_block_number)?.unwrap();
    let header = storage.get_block_header(last_block_number)?.unwrap();

    let last_block = Block { header, body };

    validate_parent_canonical(&last_block, storage)?;
    let parent_header = find_parent_header(&last_block.header, storage)?;

    let mut state = evm_state(storage.clone(), last_block.header.parent_hash);

    validate_block(&last_block, &parent_header, &state)?;

    let receipts = execute_block(&last_block, &mut state)?;

    validate_gas_used(&receipts, &last_block.header)?;

    let account_updates = get_state_transitions(&mut state);

    let mut accounts: HashMap<RevmAddress, AccountInfo> = HashMap::new();

    let mut storage_hmap: HashMap<
        RevmAddress,
        HashMap<revm::primitives::alloy_primitives::U256, revm::primitives::alloy_primitives::U256>,
    > = HashMap::new();

    // Apply the account updates over the last block's state and compute the new state root
    let new_state_root = state
        .database()
        .apply_account_updates(last_block.header.parent_hash, &account_updates)?
        .unwrap_or_default();

    // Check state root matches the one in block header after execution
    validate_state_root(&last_block.header, new_state_root)?;
    info!("LAST BLOCK STATE MATCHES");

    for account_update in account_updates {
        let address = RevmAddress::from_slice(account_update.address.as_bytes());
        let account_info = account_update.info.unwrap();

        let revm_account_info = revm::primitives::AccountInfo {
            balance: revm::primitives::alloy_primitives::U256::from_be_bytes(u64_to_u8_array(
                account_info.balance.0.as_slice(),
            )),
            nonce: account_info.nonce,
            code_hash: FixedBytes::from_slice(account_info.code_hash.as_bytes()),
            code: None, // Is this necessary?
        };

        accounts.insert(address, revm_account_info);

        let inner_storage = storage_hmap.entry(address).or_default();

        let added_storage: HashMap<H256, U256> = account_update.added_storage;
        for (key, value) in added_storage {
            // Convert H256 key to U256
            let k_u256 = U256::from_big_endian(&key.0);
            let k = revm::primitives::alloy_primitives::U256::from_be_bytes(u64_to_u8_array(
                k_u256.0.as_slice(),
            ));
            let v = revm::primitives::alloy_primitives::U256::from_be_bytes(u64_to_u8_array(
                value.0.as_slice(),
            ));
            inner_storage.insert(k, v);
        }
    }

    let oldest_block_number = state.oldest_block_number();

    let mut block_headers = Vec::new();

    for block_number in (oldest_block_number..=(last_block_number - 1)).rev() {
        let block_header = state.database().get_block_header(block_number)?.unwrap();
        block_headers.push(block_header);
    }

    let mut block_hashes: HashMap<u64, B256> = HashMap::new();

    for i in 0..block_headers.len() - 1 {
        let child_header = &block_headers[i];
        let parent_header = &block_headers[i + 1];

        let parent_hash = B256::from_slice(&child_header.parent_hash.0);
        block_hashes.insert(parent_header.number, parent_hash);
    }

    Ok(MemoryDB {
        accounts,
        storage: storage_hmap,
        block_hashes,
    })
}

fn u64_to_u8_array(u64_slice: &[u64]) -> [u8; 48] {
    // Expected exactly 6 u64 values to convert to [u8; 48]

    // 4 * u64 == 48 bytes
    let mut bytes = [0u8; 48];
    let mut index = 0;

    for &value in u64_slice {
        let byte_slice = value.to_be_bytes();
        bytes[index..index + 8].copy_from_slice(&byte_slice);
        index += 8;
    }

    bytes
}
