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
    io::{BufReader, BufWriter},
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

use risc0_zkvm::sha::{Digest, Digestible};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ProverInputData {
    pub block: Block,
    pub parent_block_header: BlockHeader,
    pub db: ExecutionDB,
}

#[derive(Debug, Clone)]
struct ProverServer {
    ip: IpAddr,
    port: u16,
    store: Store,
    eth_client: EthClient,
    on_chain_proposer_address: Address,
    verifier_address: Address,
    verifier_private_key: SecretKey,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Risc0Proof {
    pub receipt: Box<risc0_zkvm::Receipt>,
    pub prover_id: Vec<u32>,
}

impl Risc0Proof {
    pub fn new(receipt: risc0_zkvm::Receipt, prover_id: Vec<u32>) -> Self {
        Risc0Proof {
            receipt: Box::new(receipt),
            prover_id,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Sp1Proof {
    pub proof: Box<sp1_sdk::SP1ProofWithPublicValues>,
    pub vk: sp1_sdk::SP1VerifyingKey,
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
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ZkProof {
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
    /// The ZkProof has the Prover Variant implicitly.
    Submit {
        block_number: u64,
        zk_proof: ZkProof,
    },

    /// 4.
    /// The Server acknowledges the receipt of the proof and updates its state,
    SubmitAck { block_number: u64 },
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
                if let Err(e) = self
                    .handle_request(&mut stream, last_verified_block + 1)
                    .await
                {
                    warn!("Failed to handle request: {e}");
                }
            }
            Ok(ProofData::Submit {
                block_number,
                zk_proof,
            }) => {
                self.handle_submit(&mut stream, block_number)?;

                if block_number != (last_verified_block + 1) {
                    return Err(ProverServerError::Custom(format!("Prover Client submitted an invalid block_number: {block_number}. The last_proved_block is: {}", last_verified_block)));
                }

                self.handle_proof_submission(block_number, zk_proof).await?;
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
        stream: &mut TcpStream,
        block_number: u64,
    ) -> Result<(), ProverServerError> {
        debug!("Request received");

        let latest_block_number = self
            .store
            .get_latest_block_number()?
            .ok_or(ProverServerError::StorageDataIsNone)?;

        let response = if block_number > latest_block_number {
            let response = ProofData::Response {
                block_number: None,
                input: None,
            };
            warn!("Didn't send response");
            response
        } else {
            let input = self.create_prover_input(block_number)?;
            let response = ProofData::Response {
                block_number: Some(block_number),
                input: Some(input),
            };
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

        let response = ProofData::SubmitAck { block_number };
        let writer = BufWriter::new(stream);
        serde_json::to_writer(writer, &response)
            .map_err(|e| ProverServerError::ConnectionError(e.into()))
    }

    async fn handle_proof_submission(
        &self,
        block_number: u64,
        zk_proof: ZkProof,
    ) -> Result<(), ProverServerError> {
        match zk_proof {
            ZkProof::RISC0(risc0_proof) => {
                self.handle_risc0_zkproof(block_number, risc0_proof).await?
            }
            ZkProof::SP1(sp1_proof) => self.handle_sp1_zkproof(block_number, sp1_proof).await?,
        };

        Ok(())
    }

    async fn handle_risc0_zkproof(
        &self,
        block_number: u64,
        risc0_proof: Risc0Proof,
    ) -> Result<(), ProverServerError> {
        let Risc0Proof { receipt, prover_id } = risc0_proof;
        // Send Tx
        // If we run the prover_client with RISC0_DEV_MODE=0 we will have a groth16 proof
        // Else, we will have a fake proof.
        //
        // The RISC0_DEV_MODE=1 should only be used with DEPLOYER_CONTRACT_VERIFIER=0xAA
        let seal = match receipt.inner.groth16() {
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
            *b = *prover_id.get(i).ok_or(ProverServerError::Custom(
                "Failed to get image_id in handle_proof_submission()".to_owned(),
            ))?;
        }

        let image_id: risc0_zkvm::sha::Digest = image_id.into();

        let journal_digest = Digestible::digest(&receipt.journal);

        self.send_risc0_proof(block_number, &seal, image_id, journal_digest)
            .await?;

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

    pub async fn send_risc0_proof(
        &self,
        block_number: u64,
        seal: &[u8],
        image_id: Digest,
        journal_digest: Digest,
    ) -> Result<H256, ProverServerError> {
        debug!("Sending proof for {block_number}");
        let mut calldata = Vec::new();

        // IOnChainProposer
        // function verify(uint256,bytes,bytes32,bytes32)
        // Verifier
        // function verify(bytes,bytes32,bytes32)
        // blockNumber, seal, imageId, journalDigest
        // From crates/l2/contracts/l1/interfaces/IOnChainProposer.sol
        let verify_proof_selector = keccak(b"verify(uint256,bytes,bytes32,bytes32)")
            .as_bytes()
            .get(..4)
            .ok_or(ProverServerError::Custom(
                "Failed to get verify_proof_selector in send_proof()".to_owned(),
            ))?
            .to_vec();
        calldata.extend(verify_proof_selector);

        // The calldata has to be structured in the following way:
        // block_number
        // size in bytes
        // image_id digest
        // journal digest
        // size of seal
        // seal

        // extend with block_number
        calldata.extend(H256::from_low_u64_be(block_number).as_bytes());

        // extend with size in bytes
        // 4 u256 goes after this field so: 0x80 == 128bytes == 32bytes * 4
        calldata.extend(H256::from_low_u64_be(4 * 32).as_bytes());

        // extend with image_id
        calldata.extend(image_id.as_bytes());

        // extend with journal_digest
        calldata.extend(journal_digest.as_bytes());

        // extend with size of seal
        calldata.extend(
            H256::from_low_u64_be(seal.len().try_into().map_err(|err| {
                ProverServerError::Custom(format!("Seal length does not fit in u64: {}", err))
            })?)
            .as_bytes(),
        );
        // extend with seal
        calldata.extend(seal);
        // extend with zero padding
        let leading_zeros = 32 - ((calldata.len() - 4) % 32);
        calldata.extend(vec![0; leading_zeros]);

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

    pub async fn handle_sp1_zkproof(
        &self,
        _block_number: u64,
        _sp1_proof: Sp1Proof,
    ) -> Result<(), ProverServerError> {
        todo!()
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
            // blockNumber, seal, imageId, journalDigest, programVKey, publicValues, proofBytes
            // From crates/l2/contracts/l1/interfaces/IOnChainProposer.sol
            let mut calldata = keccak(b"verify(uint256,bytes,bytes32,bytes32,bytes32,bytes,bytes)")
                .as_bytes()
                .get(..4)
                .ok_or(ProverServerError::Custom(
                    "Failed to get verify_proof_selector in send_proof()".to_owned(),
                ))?
                .to_vec();
            calldata.extend(H256::from_low_u64_be(last_verified_block + 1).as_bytes());
            // 7 inputs * 32bytes offset
            calldata.extend(H256::from_low_u64_be(7 * 32).as_bytes());
            // extend with size of the first bytes variable -> 32bytes
            calldata.extend(H256::from_low_u64_be(32).as_bytes());
            // extend with bytes, bytes32, bytes32, bytes32
            for _ in 0..=3 {
                calldata.extend(H256::zero().as_bytes());
            }
            // extend with size of the second bytes variable -> 32bytes
            calldata.extend(H256::from_low_u64_be(32).as_bytes());
            // Fill with zeroes
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
