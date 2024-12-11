use super::errors::{ProverServerError, SigIntError};
use crate::utils::{
    config::{
        committer::CommitterConfig, errors::ConfigError, eth::EthConfig,
        prover_server::ProverServerConfig,
    },
    eth_client::{eth_sender::Overrides, EthClient, WrappedTransaction},
};
use ethrex_core::{
    types::{Block, BlockHeader},
    Address, H256,
};
use ethrex_storage::Store;
use ethrex_vm::{execution_db::ExecutionDB, EvmError};
use keccak_hash::keccak;
use secp256k1::SecretKey;
use serde::{Deserialize, Serialize};
use std::{
    io::{BufReader, BufWriter, Write},
    net::{IpAddr, Shutdown, TcpListener, TcpStream},
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};
use tokio::{
    signal::unix::{signal, SignalKind},
    time::sleep,
};
use tracing::{debug, error, info, warn};

use risc0_zkvm::sha::Digestible;
use sp1_sdk::HashableKey;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ProverInputData {
    pub block: Block,
    pub parent_block_header: BlockHeader,
    pub db: ExecutionDB,
}

#[derive(Clone)]
struct ProverServer {
    ip: IpAddr,
    port: u16,
    store: Store,
    eth_client: EthClient,
    on_chain_proposer_address: Address,
    verifier_address: Address,
    verifier_private_key: SecretKey,
}

#[derive(Debug, Clone, Copy)]
/// Enum used to identify the different proving systems.
pub enum ProverType {
    RISC0,
    SP1,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Risc0Proof {
    pub receipt: Box<risc0_zkvm::Receipt>,
    pub prover_id: Vec<u32>,
}

pub struct Risc0ContractData {
    pub block_proof: Vec<u8>,
    pub image_id: Vec<u8>,
    pub journal_digest: Vec<u8>,
}

impl Risc0Proof {
    pub fn new(receipt: risc0_zkvm::Receipt, prover_id: Vec<u32>) -> Self {
        Risc0Proof {
            receipt: Box::new(receipt),
            prover_id,
        }
    }

    pub fn contract_data(&self) -> Result<Risc0ContractData, ProverServerError> {
        // If we run the prover_client with RISC0_DEV_MODE=0 we will have a groth16 proof
        // Else, we will have a fake proof.
        //
        // The RISC0_DEV_MODE=1 should only be used with DEPLOYER_CONTRACT_VERIFIER=0xAA
        let block_proof = match self.receipt.inner.groth16() {
            Ok(inner) => {
                // The SELECTOR is used to perform an extra check inside the groth16 verifier contract.
                let mut selector =
                    hex::encode(inner.verifier_parameters.as_bytes().get(..4).ok_or(
                        ProverServerError::Custom(
                            "Failed to get verify_proof_selector in send_proof()".to_owned(),
                        ),
                    )?);
                let seal = hex::encode(inner.clone().seal);
                selector.push_str(&seal);
                hex::decode(selector).map_err(|e| {
                    ProverServerError::Custom(format!("Failed to hex::decode(selector): {e}"))
                })?
            }
            Err(_) => vec![32; 0],
        };

        let mut image_id: [u32; 8] = [0; 8];
        for (i, b) in image_id.iter_mut().enumerate() {
            *b = *self.prover_id.get(i).ok_or(ProverServerError::Custom(
                "Failed to get image_id in handle_proof_submission()".to_owned(),
            ))?;
        }

        let image_id: risc0_zkvm::sha::Digest = image_id.into();
        let image_id = image_id.as_bytes().to_vec();

        let journal_digest = Digestible::digest(&self.receipt.journal)
            .as_bytes()
            .to_vec();

        Ok(Risc0ContractData {
            block_proof,
            image_id,
            journal_digest,
        })
    }

    pub fn contract_data_empty() -> Risc0ContractData {
        Risc0ContractData {
            block_proof: vec![0; 32],
            image_id: vec![0; 32],
            journal_digest: vec![0; 32],
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Sp1Proof {
    pub proof: Box<sp1_sdk::SP1ProofWithPublicValues>,
    pub vk: sp1_sdk::SP1VerifyingKey,
}

pub struct Sp1ContractData {
    pub public_values: Vec<u8>,
    pub vk: Vec<u8>,
    pub proof_bytes: Vec<u8>,
}

impl Sp1Proof {
    pub fn new(
        proof: sp1_sdk::SP1ProofWithPublicValues,
        verifying_key: sp1_sdk::SP1VerifyingKey,
    ) -> Self {
        Sp1Proof {
            proof: Box::new(proof),
            vk: verifying_key,
        }
    }

    pub fn contract_data(&self) -> Result<Sp1ContractData, ProverServerError> {
        let vk = self
            .vk
            .bytes32()
            .strip_prefix("0x")
            .ok_or(ProverServerError::Custom(
                "Failed to strip_prefix of sp1 vk".to_owned(),
            ))?
            .to_string();
        let vk_bytes = hex::decode(&vk)
            .map_err(|_| ProverServerError::Custom("Failed hex::decode(&vk)".to_owned()))?;

        Ok(Sp1ContractData {
            public_values: self.proof.public_values.to_vec(),
            vk: vk_bytes,
            proof_bytes: self.proof.bytes(),
        })
    }

    // TODO: better way of giving empty information
    pub fn contract_data_empty() -> Sp1ContractData {
        Sp1ContractData {
            public_values: vec![0; 32],
            vk: vec![0; 32],
            proof_bytes: vec![0; 32],
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ProvingOutput {
    RISC0(Risc0Proof),
    SP1(Sp1Proof),
}

/// Enum for the ProverServer <--> ProverClient Communication Protocol.
#[derive(Serialize, Deserialize)]
pub enum ProofData {
    /// 1.
    /// The Client initiates the connection with a Request.
    /// Asking for the ProverInputData the prover_server considers/needs.
    Request,

    /// 2.
    /// The Server responds with a Response containing the ProverInputData.
    /// If the Response will is ProofData::Response{None, None}, the Client knows that the Request couldn't be performed.
    Response {
        block_number: Option<u64>,
        input: Option<ProverInputData>,
    },

    /// 3.
    /// The Client submits the zk Proof generated by the prover
    /// for the specified block.
    /// The [ProvingOutput] has the [ProverType] implicitly.
    Submit {
        block_number: u64,
        proving_output: ProvingOutput,
    },

    /// 4.
    /// The Server acknowledges the receipt of the proof and updates its state,
    SubmitAck { block_number: u64 },
}

impl ProofData {
    /// Builder function for creating a Request
    pub fn request() -> Self {
        ProofData::Request
    }

    /// Builder function for creating a Response
    pub fn response(block_number: Option<u64>, input: Option<ProverInputData>) -> Self {
        ProofData::Response {
            block_number,
            input,
        }
    }

    /// Builder function for creating a Submit
    pub fn submit(block_number: u64, proving_output: ProvingOutput) -> Self {
        ProofData::Submit {
            block_number,
            proving_output,
        }
    }

    /// Builder function for creating a SubmitAck
    pub fn submit_ack(block_number: u64) -> Self {
        ProofData::SubmitAck { block_number }
    }
}

pub async fn start_prover_server(store: Store) -> Result<(), ConfigError> {
    let server_config = ProverServerConfig::from_env()?;
    let eth_config = EthConfig::from_env()?;
    let proposer_config = CommitterConfig::from_env()?;
    let mut prover_server =
        ProverServer::new_from_config(server_config.clone(), &proposer_config, eth_config, store)
            .await?;
    prover_server.run(&server_config).await;
    Ok(())
}

impl ProverServer {
    pub async fn new_from_config(
        config: ProverServerConfig,
        committer_config: &CommitterConfig,
        eth_config: EthConfig,
        store: Store,
    ) -> Result<Self, ConfigError> {
        let eth_client = EthClient::new(&eth_config.rpc_url);
        let on_chain_proposer_address = committer_config.on_chain_proposer_address;

        Ok(Self {
            ip: config.listen_ip,
            port: config.listen_port,
            store,
            eth_client,
            on_chain_proposer_address,
            verifier_address: config.verifier_address,
            verifier_private_key: config.verifier_private_key,
        })
    }

    pub async fn run(&mut self, server_config: &ProverServerConfig) {
        loop {
            let result = if server_config.dev_mode {
                self.main_logic_dev().await
            } else {
                self.clone().main_logic(server_config).await
            };

            match result {
                Ok(_) => {
                    if !server_config.dev_mode {
                        warn!("Prover Server shutting down");
                        break;
                    }
                }
                Err(e) => {
                    let error_message = if !server_config.dev_mode {
                        format!("Prover Server, severe Error, trying to restart the main_logic function: {e}")
                    } else {
                        format!("Prover Server Dev Error: {e}")
                    };
                    error!(error_message);
                }
            }

            sleep(Duration::from_millis(200)).await;
        }
    }

    async fn main_logic(
        mut self,
        server_config: &ProverServerConfig,
    ) -> Result<(), ProverServerError> {
        let (tx, rx) = mpsc::channel();

        // It should never exit the start() fn, handling errors inside the for loop of the function.
        let server_handle = tokio::spawn(async move { self.start(rx).await });

        ProverServer::handle_sigint(tx, server_config).await?;

        match server_handle.await {
            Ok(result) => match result {
                Ok(_) => (),
                Err(e) => return Err(e),
            },
            Err(e) => return Err(e.into()),
        };

        Ok(())
    }

    async fn handle_sigint(
        tx: mpsc::Sender<()>,
        config: &ProverServerConfig,
    ) -> Result<(), ProverServerError> {
        let mut sigint = signal(SignalKind::interrupt())?;
        sigint.recv().await.ok_or(SigIntError::Recv)?;
        tx.send(()).map_err(SigIntError::Send)?;
        TcpStream::connect(format!("{}:{}", config.listen_ip, config.listen_port))?
            .shutdown(Shutdown::Both)
            .map_err(SigIntError::Shutdown)?;

        Ok(())
    }

    pub async fn start(&mut self, rx: Receiver<()>) -> Result<(), ProverServerError> {
        let listener = TcpListener::bind(format!("{}:{}", self.ip, self.port))?;

        info!("Starting TCP server at {}:{}", self.ip, self.port);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    debug!("Connection established!");

                    if let Ok(()) = rx.try_recv() {
                        info!("Shutting down Prover Server");
                        break;
                    }

                    if let Err(e) = self.handle_connection(stream).await {
                        error!("Error handling connection: {}", e);
                    }
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
        Ok(())
    }

    async fn handle_connection(&mut self, mut stream: TcpStream) -> Result<(), ProverServerError> {
        let buf_reader = BufReader::new(&stream);

        let last_verified_block =
            EthClient::get_last_verified_block(&self.eth_client, self.on_chain_proposer_address)
                .await?;

        let last_verified_block = if last_verified_block == u64::MAX {
            0
        } else {
            last_verified_block
        };

        let data: Result<ProofData, _> = serde_json::de::from_reader(buf_reader);
        match data {
            Ok(ProofData::Request) => {
                if let Err(e) = self.handle_request(&stream, last_verified_block + 1).await {
                    warn!("Failed to handle request: {e}");
                }
            }
            Ok(ProofData::Submit {
                block_number,
                proving_output,
            }) => {
                self.handle_submit(&mut stream, block_number)?;

                if block_number != (last_verified_block + 1) {
                    return Err(ProverServerError::Custom(format!("Prover Client submitted an invalid block_number: {block_number}. The last_proved_block is: {}", last_verified_block)));
                }

                self.handle_proof_submission(block_number, proving_output)
                    .await?;
            }
            Err(e) => {
                warn!("Failed to parse request: {e}");
            }
            _ => {
                warn!("Invalid request");
            }
        }

        debug!("Connection closed");
        Ok(())
    }

    async fn handle_request(
        &self,
        stream: &TcpStream,
        block_number: u64,
    ) -> Result<(), ProverServerError> {
        debug!("Request received");

        let latest_block_number = self
            .store
            .get_latest_block_number()?
            .ok_or(ProverServerError::StorageDataIsNone)?;

        let response = if block_number > latest_block_number {
            let response = ProofData::response(None, None);
            warn!("Didn't send response");
            response
        } else {
            let input = self.create_prover_input(block_number)?;
            let response = ProofData::response(Some(block_number), Some(input));
            info!("Sent Response for block_number: {block_number}");
            response
        };

        let writer = BufWriter::new(stream);
        serde_json::to_writer(writer, &response)
            .map_err(|e| ProverServerError::ConnectionError(e.into()))
    }

    fn handle_submit(
        &self,
        stream: &mut TcpStream,
        block_number: u64,
    ) -> Result<(), ProverServerError> {
        debug!("Submit received for BlockNumber: {block_number}");

        let response = ProofData::submit_ack(block_number);
        let json_string = serde_json::to_string(&response)
            .map_err(|e| ProverServerError::Custom(format!("serde_json::to_string(): {e}")))?;
        stream
            .write_all(json_string.as_bytes())
            .map_err(ProverServerError::ConnectionError)?;

        Ok(())
    }

    fn create_prover_input(&self, block_number: u64) -> Result<ProverInputData, ProverServerError> {
        let header = self
            .store
            .get_block_header(block_number)?
            .ok_or(ProverServerError::StorageDataIsNone)?;
        let body = self
            .store
            .get_block_body(block_number)?
            .ok_or(ProverServerError::StorageDataIsNone)?;

        let block = Block::new(header, body);

        let db = ExecutionDB::from_exec(&block, &self.store).map_err(EvmError::ExecutionDB)?;

        let parent_block_header = self
            .store
            .get_block_header_by_hash(block.header.parent_hash)?
            .ok_or(ProverServerError::StorageDataIsNone)?;

        debug!("Created prover input for block {block_number}");

        Ok(ProverInputData {
            db,
            block,
            parent_block_header,
        })
    }

    pub async fn handle_proof_submission(
        &self,
        block_number: u64,
        proving_output: ProvingOutput,
    ) -> Result<H256, ProverServerError> {
        // TODO:
        // Ideally we should wait to have both proofs
        // We will have to send them in the same transaction.
        let (sp1_contract_data, risc0_contract_data) = match proving_output {
            ProvingOutput::RISC0(risc0_proof) => {
                let risc0_contract_data = risc0_proof.contract_data()?;
                let sp1_contract_data = Sp1Proof::contract_data_empty();
                (sp1_contract_data, risc0_contract_data)
            }
            ProvingOutput::SP1(sp1_proof) => {
                let risc0_contract_data = Risc0Proof::contract_data_empty();
                let sp1_contract_data = sp1_proof.contract_data()?;
                (sp1_contract_data, risc0_contract_data)
            }
        };

        debug!("Sending proof for {block_number}");

        // IOnChainProposer
        // function verify(uint256,bytes,bytes32,bytes32,bytes32,bytes,bytes)
        // blockNumber, blockProof, imageId, journalDigest, programVKey, publicValues, proofBytes
        // From crates/l2/contracts/l1/interfaces/IOnChainProposer.sol
        let mut calldata = keccak(b"verify(uint256,bytes,bytes32,bytes32,bytes32,bytes,bytes)")
            .as_bytes()
            .get(..4)
            .ok_or(ProverServerError::Custom(
                "Failed to get verify_proof_selector in send_proof()".to_owned(),
            ))?
            .to_vec();

        // The calldata has to be structured in the following way:
        // block_number
        // offset of first bytes parameter
        // image_id
        // journal
        // programVKey
        // offset of second bytes parameter
        // offset of third bytes parameter
        // size of block_proof
        // block_proof
        // size of publicValues
        // publicValues
        // size of proofBytes
        // proofBytes

        // extend with block_number
        calldata.extend(H256::from_low_u64_be(block_number).as_bytes());

        // extend with offset in bytes
        calldata.extend(H256::from_low_u64_be(7 * 32).as_bytes());

        // extend with image_id
        calldata.extend(risc0_contract_data.image_id);

        // extend with journal_digest
        calldata.extend(risc0_contract_data.journal_digest);

        // extend with program_vkey
        calldata.extend(sp1_contract_data.vk);

        // extend with offset in bytes of second bytes parameter
        let block_proof_len: u64 =
            risc0_contract_data
                .block_proof
                .len()
                .try_into()
                .map_err(|err| {
                    ProverServerError::Custom(format!(
                        "calldata length does not fit in u64: {}",
                        err
                    ))
                })?;

        let calldata_len: u64 = (calldata.len()).try_into().map_err(|err| {
            ProverServerError::Custom(format!("calldata length does not fit in u64: {}", err))
        })?;

        let leading_zeros_after_block_proof: u64 =
            calculate_padding(calldata_len + 64 + block_proof_len)?
                .try_into()
                .map_err(|err| {
                    ProverServerError::Custom(format!(
                        "calculate_padding length does not fit in u64: {}",
                        err
                    ))
                })?;

        // 2 * 32 bytes are the offset of the second and third bytes offsets
        // and then 32 bytes more for the len of block_proof
        let bytes = 32 * 3;
        let offset = calldata_len + block_proof_len + leading_zeros_after_block_proof + bytes;
        calldata.extend(H256::from_low_u64_be(offset - 4).as_bytes());

        // add 32 bytes to reflect the last extend()
        let offset = offset + 32;
        let public_values_len: u64 =
            sp1_contract_data
                .public_values
                .len()
                .try_into()
                .map_err(|err| {
                    ProverServerError::Custom(format!(
                        "public_values length does not fit in u64: {}",
                        err
                    ))
                })?;

        let leading_zeros_after_public_values: u64 = calculate_padding(offset + public_values_len)?
            .try_into()
            .map_err(|err| {
                ProverServerError::Custom(format!(
                    "calculate_padding length does not fit in u64: {}",
                    err
                ))
            })?;

        let offset = offset + public_values_len + leading_zeros_after_public_values - 4;
        // extend with offset in bytes of third bytes parameter
        calldata.extend(H256::from_low_u64_be(offset).as_bytes());

        // extend with size of block_proof and block_proof
        extend_calldata_with_bytes(&mut calldata, &risc0_contract_data.block_proof)?;

        // extend with size of public_values and public_values
        extend_calldata_with_bytes(&mut calldata, &sp1_contract_data.public_values)?;

        // extend with size of proof_bytes and proof_bytes
        extend_calldata_with_bytes(&mut calldata, &sp1_contract_data.proof_bytes)?;

        let verify_tx = self
            .eth_client
            .build_eip1559_transaction(
                self.on_chain_proposer_address,
                self.verifier_address,
                calldata.into(),
                Overrides::default(),
                10,
            )
            .await?;

        let verify_tx_hash = self
            .eth_client
            .send_wrapped_transaction_with_retry(
                &WrappedTransaction::EIP1559(verify_tx),
                &self.verifier_private_key,
                3 * 60,
                10,
            )
            .await?;

        info!("Sent proof for block {block_number}, with transaction hash {verify_tx_hash:#x}");

        Ok(verify_tx_hash)
    }

    pub async fn main_logic_dev(&self) -> Result<(), ProverServerError> {
        loop {
            thread::sleep(Duration::from_millis(200));

            let last_committed_block = EthClient::get_last_committed_block(
                &self.eth_client,
                self.on_chain_proposer_address,
            )
            .await?;

            let last_verified_block = EthClient::get_last_verified_block(
                &self.eth_client,
                self.on_chain_proposer_address,
            )
            .await?;

            if last_committed_block == u64::MAX {
                debug!("No blocks commited yet");
                continue;
            }

            if last_committed_block == last_verified_block {
                debug!("No new blocks to prove");
                continue;
            }

            info!("Last committed: {last_committed_block} - Last verified: {last_verified_block}");

            // IOnChainProposer
            // function verify(uint256,bytes,bytes32,bytes32,bytes32,bytes,bytes)
            // blockNumber, blockProof, imageId, journalDigest, programVKey, publicValues, proofBytes
            // From crates/l2/contracts/l1/interfaces/IOnChainProposer.sol

            // The calldata has to be structured in the following way:
            // block_number
            // offset of first bytes parameter
            // image_id
            // journal
            // programVKey
            // offset of second bytes parameter
            // offset of third bytes parameter
            // size of block_proof
            // block_proof
            // size of publicValues
            // publicValues
            // size of proofBytes
            // proofBytes
            let mut calldata = keccak(b"verify(uint256,bytes,bytes32,bytes32,bytes32,bytes,bytes)")
                .as_bytes()
                .get(..4)
                .ok_or(ProverServerError::Custom(
                    "Failed to get verify_proof_selector in send_proof()".to_owned(),
                ))?
                .to_vec();
            calldata.extend(H256::from_low_u64_be(last_verified_block + 1).as_bytes());
            // offset of first bytes parameter
            calldata.extend(H256::from_low_u64_be(7 * 32).as_bytes());
            // extend with bytes32, bytes32, bytes32
            for _ in 0..=3 {
                calldata.extend(H256::zero().as_bytes());
            }
            // offset of second bytes parameter
            calldata.extend(H256::zero().as_bytes());
            // offset of third bytes parameter
            calldata.extend(H256::zero().as_bytes());
            // extend with size of the third bytes variable -> 32bytes
            calldata.extend(H256::from_low_u64_be(32).as_bytes());
            calldata.extend(H256::zero().as_bytes());

            let verify_tx = self
                .eth_client
                .build_eip1559_transaction(
                    self.on_chain_proposer_address,
                    self.verifier_address,
                    calldata.into(),
                    Overrides {
                        ..Default::default()
                    },
                    10,
                )
                .await?;

            info!("Sending verify transaction.");

            let verify_tx_hash = self
                .eth_client
                .send_wrapped_transaction_with_retry(
                    &WrappedTransaction::EIP1559(verify_tx),
                    &self.verifier_private_key,
                    3 * 60,
                    10,
                )
                .await?;

            info!("Sent proof for block {last_verified_block}, with transaction hash {verify_tx_hash:#x}");

            info!(
                "Mocked verify transaction sent for block {}",
                last_verified_block + 1
            );
        }
    }
}

pub fn extend_calldata_with_bytes(
    calldata: &mut Vec<u8>,
    bytes: &[u8],
) -> Result<(), ProverServerError> {
    // extend with size of bytes
    calldata.extend(
        H256::from_low_u64_be(bytes.len().try_into().map_err(|err| {
            ProverServerError::Custom(format!("bytes length does not fit in u64: {}", err))
        })?)
        .as_bytes(),
    );
    // extend with bytes
    calldata.extend(bytes);
    // extend with zero padding
    let calldata_len: u64 = calldata.len().try_into().map_err(|err| {
        ProverServerError::Custom(format!("calldata length does not fit in u64: {}", err))
    })?;
    let leading_zeros = calculate_padding(calldata_len)?;
    calldata.extend(vec![0; leading_zeros]);

    Ok(())
}

fn calculate_padding(calldata_len: u64) -> Result<usize, ProverServerError> {
    let len = calldata_len - 4;

    // Calculate leading zeros needed for alignment to 32 bytes
    let leading_zeros = if len % 32 == 0 { 0 } else { 32 - (len % 32) };
    leading_zeros
        .try_into()
        .map_err(|_| ProverServerError::Custom("Failed to calculate padding".to_owned()))
}

// used for debugging purposes
#[allow(unused)]
fn print_calldata(calldata: Vec<u8>) {
    let mut hex_string = String::new();
    for byte in calldata {
        hex_string.push_str(&format!("{:02x}", byte));
    }
    println!("CALLDATA: {}", hex_string);
}
