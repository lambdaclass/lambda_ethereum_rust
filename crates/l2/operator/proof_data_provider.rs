use serde::{Deserialize, Serialize};
use sp1_sdk::SP1ProofWithPublicValues;
use std::{
    collections::HashMap,
    io::{BufReader, BufWriter},
    net::{IpAddr, TcpListener, TcpStream},
};
use tracing::{debug, info, warn};

use ethereum_rust_blockchain::{
    find_parent_header, validate_block, validate_gas_used, validate_parent_canonical,
    validate_state_root,
};
use ethereum_rust_core::types::Block;
use ethereum_rust_storage::Store;
use ethereum_rust_vm::{evm_state, execute_block, get_state_transitions, RevmAddress};
use ethereum_types::{H256, U256};
use prover_lib::{
    db_memorydb::MemoryDB,
    inputs::{ProverInput, ProverInputNoExecution},
};

use crate::prover::zk_prover::ProverMode;

use crate::utils::config::proof_data_provider::ProofDataProviderConfig;
use revm::primitives::{AccountInfo, FixedBytes, B256};

use super::errors::ProofDataProviderError;

pub async fn start_proof_data_provider(store: Store) {
    let config = ProofDataProviderConfig::from_env().expect("ProofDataProviderConfig::from_env()");
    let proof_data_provider = ProofDataProvider::new_from_config(config, store);
    proof_data_provider
        .start()
        .await
        .expect("proof_data_provider.start()");
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
    pub fn new_from_config(config: ProofDataProviderConfig, store: Store) -> Self {
        Self {
            ip: config.listen_ip,
            port: config.listen_port,
            store,
        }
    }

    pub async fn start(&self) -> Result<(), ProofDataProviderError> {
        let listener = TcpListener::bind(format!("{}:{}", self.ip, self.port))?;

        let mut last_proved_block = 0;

        info!("Starting TCP server at {}:{}", self.ip, self.port);
        for stream in listener.incoming() {
            debug!("Connection established!");
            self.handle_connection(stream?, &mut last_proved_block)
                .await;
        }
        Ok(())
    }

    async fn handle_connection(&self, mut stream: TcpStream, last_proved_block: &mut u64) {
        let buf_reader = BufReader::new(&stream);

        let data: Result<ProofData, _> = serde_json::de::from_reader(buf_reader);
        match data {
            Ok(ProofData::Request { mode }) => {
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

    async fn handle_request(
        &self,
        stream: &mut TcpStream,
        prover_mode: ProverMode,
        last_proved_block: u64,
    ) -> Result<(), String> {
        debug!("Request received");

        // Build Inputs
        //let mut blocks = read_chain_file("./test_data/chain.rlp");
        //let head_block = blocks.pop().unwrap();
        //let parent_block_header = blocks.pop().unwrap().header;
        //
        //let _prover_inputs_verification = ProverInputNoExecution {
        //    head_block: head_block.clone(),
        //    parent_block_header: parent_block_header.clone(),
        //    block_is_valid: false,
        //};

        let last_block_number = self.store.get_latest_block_number().unwrap().unwrap();
        let body = self
            .store
            .get_block_body(last_block_number)
            .unwrap()
            .unwrap();
        let header = self
            .store
            .get_block_header(last_block_number)
            .unwrap()
            .unwrap();

        let last_block = Block { header, body };

        let parent_block_header = self
            .store
            .get_block_header(last_block_number - 1)
            .unwrap()
            .unwrap();

        // we need the storage of the current_block-1
        // we should execute the EVM with that state and simulating the inclusion of the current_block
        let memory_db =
            get_last_block_state(&self.store, &last_block).map_err(|e| format!("Error: {e}"))?;
        // with the execution_outputs we should get a way to have the State/Store represented with Hashmaps.
        // finally, this information has to be contained in anstructure that can be de/serealized,
        // so that, any zkVM could receive the state as input and prove the block execution.

        let prover_inputs_execution = ProverInput {
            block: last_block,
            parent_block_header,
            db: memory_db,
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
        debug!("Submit received. ID: {id}, proof: {:?}", proof.proof);

        let response = ProofData::SubmitAck { id };
        let writer = BufWriter::new(stream);
        serde_json::to_writer(writer, &response).map_err(|e| e.to_string())
    }
}

/// Same concept as adding a block [ethereum_rust_blockchain::add_block].
fn get_last_block_state(
    storage: &Store,
    last_block: &Block,
) -> Result<MemoryDB, Box<dyn std::error::Error>> {
    validate_parent_canonical(last_block, storage)?;
    let parent_header = find_parent_header(&last_block.header, storage)?;

    let mut state = evm_state(storage.clone(), last_block.header.parent_hash);

    validate_block(last_block, &parent_header, &state)?;

    let receipts = execute_block(last_block, &mut state)?;

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

    let last_block_number = last_block.header.number;
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
