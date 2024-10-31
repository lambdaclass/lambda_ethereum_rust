use crate::utils::{
    config::{eth::EthConfig, proposer::ProposerConfig, read_env_file},
    eth_client::EthClient,
    merkle_tree::merkelize,
};
use bytes::Bytes;
use errors::ProposerError;
use ethereum_rust_blockchain::constants::TX_GAS_COST;
use ethereum_rust_core::types::{
    Block, EIP1559Transaction, GenericTransaction, PrivilegedL2Transaction, PrivilegedTxType,
    Transaction, TxKind,
};
use ethereum_rust_dev::utils::engine_client::{config::EngineApiConfig, EngineClient};
use ethereum_rust_rpc::types::fork_choice::{ForkChoiceState, PayloadAttributesV3};
use ethereum_rust_storage::Store;
use ethereum_rust_vm::{evm_state, execute_block, get_state_transitions};
use ethereum_types::{Address, H256, U256};
use keccak_hash::keccak;
use lambdaworks_crypto::commitments::kzg::StructuredReferenceString;
use lambdaworks_math::{
    elliptic_curve::short_weierstrass::{
        curves::bls12_381::{
            curve::BLS12381Curve, default_types::FrElement, twist::BLS12381TwistCurve,
        },
        point::ShortWeierstrassProjectivePoint,
        traits::Compress,
    },
    msm::pippenger::msm,
    traits::ByteConversion,
    unsigned_integer::element::UnsignedInteger,
};
use libsecp256k1::SecretKey;
use state_diff::{AccountStateDiff, DepositLog, StateDiff, WithdrawalLog};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time::sleep;
use tracing::{error, info, warn};

pub mod l1_watcher;
pub mod prover_server;
pub mod state_diff;

pub mod errors;

const COMMIT_FUNCTION_SELECTOR: [u8; 4] = [132, 97, 12, 179];
const VERIFY_FUNCTION_SELECTOR: [u8; 4] = [133, 133, 44, 228];

pub struct Proposer {
    eth_client: EthClient,
    engine_client: EngineClient,
    on_chain_proposer_address: Address,
    l1_address: Address,
    l1_private_key: SecretKey,
    block_production_interval: Duration,
    srs: StructuredReferenceString<
        ShortWeierstrassProjectivePoint<BLS12381Curve>,
        ShortWeierstrassProjectivePoint<BLS12381TwistCurve>,
    >,
}

pub async fn start_proposer(store: Store) {
    info!("Starting Proposer");

    if let Err(e) = read_env_file() {
        warn!("Failed to read .env file: {e}");
    }

    let l1_watcher = tokio::spawn(l1_watcher::start_l1_watcher(store.clone()));
    let prover_server = tokio::spawn(prover_server::start_prover_server(store.clone()));
    let proposer = tokio::spawn(async move {
        let eth_config = EthConfig::from_env().expect("EthConfig::from_env");
        let proposer_config = ProposerConfig::from_env().expect("ProposerConfig::from_env");
        let engine_config = EngineApiConfig::from_env().expect("EngineApiConfig::from_env");
        let proposer = Proposer::new_from_config(&proposer_config, eth_config, engine_config)
            .expect("Proposer::new_from_config");
        let head_block_hash = {
            let current_block_number = store
                .get_latest_block_number()
                .expect("store.get_latest_block_number")
                .expect("store.get_latest_block_number returned None");
            store
                .get_canonical_block_hash(current_block_number)
                .expect("store.get_canonical_block_hash")
                .expect("store.get_canonical_block_hash returned None")
        };
        proposer
            .start(head_block_hash, store)
            .await
            .expect("Proposer::start");
    });
    tokio::try_join!(l1_watcher, prover_server, proposer).expect("tokio::try_join");
}

fn load_g1_points(
    path: &str,
) -> Result<Vec<ShortWeierstrassProjectivePoint<BLS12381Curve>>, ProposerError> {
    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);
    let mut g1_points = vec![];
    let mut buf = [0u8; 48];
    while let Ok(_) = reader.read_exact(&mut buf) {
        g1_points.push(BLS12381Curve::decompress_g1_point(&mut buf).unwrap());
    }

    Ok(g1_points)
}

fn load_g2_points(
    path: &str,
) -> Result<Vec<ShortWeierstrassProjectivePoint<BLS12381TwistCurve>>, ProposerError> {
    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);
    let mut buf = [0u8; 96];
    let mut g2_points = vec![];
    while let Ok(_) = reader.read_exact(&mut buf) {
        g2_points.push(BLS12381Curve::decompress_g2_point(&mut buf).unwrap());
    }

    Ok(g2_points)
}

impl Proposer {
    pub fn new_from_config(
        proposer_config: &ProposerConfig,
        eth_config: EthConfig,
        engine_config: EngineApiConfig,
    ) -> Result<Self, ProposerError> {
        let g1_points = load_g1_points(&proposer_config.g1_points_path)?;
        let g2_points = load_g2_points(&proposer_config.g2_points_path)?;

        let srs = StructuredReferenceString::new(
            g1_points.as_slice(),
            &[g2_points[0].clone(), g2_points[1].clone()],
        );

        Ok(Self {
            eth_client: EthClient::new(&eth_config.rpc_url),
            engine_client: EngineClient::new_from_config(engine_config)?,
            on_chain_proposer_address: proposer_config.on_chain_proposer_address,
            l1_address: proposer_config.l1_address,
            l1_private_key: proposer_config.l1_private_key,
            block_production_interval: Duration::from_millis(proposer_config.interval_ms),
            srs,
        })
    }

    pub async fn start(&self, head_block_hash: H256, store: Store) -> Result<(), ProposerError> {
        let mut head_block_hash = head_block_hash;
        loop {
            head_block_hash = self.produce_block(head_block_hash).await?;

            // TODO: Check what happens with the transactions included in the payload of the failed block.
            if head_block_hash == H256::zero() {
                error!("Failed to produce block");
                continue;
            }

            let block = store
                .get_block_by_hash(head_block_hash)
                .map_err(|error| {
                    ProposerError::FailedToRetrieveBlockFromStorage(error.to_string())
                })?
                .ok_or(ProposerError::FailedToProduceBlock(
                    "Failed to get block by hash from storage".to_string(),
                ))?;

            let withdrawals = self.get_block_withdrawals(&block)?;
            let deposits = self.get_block_deposits(&block)?;

            let withdrawal_logs_merkle_root = self.get_withdrawals_merkle_root(
                withdrawals.iter().map(|(hash, _tx)| hash.clone()).collect(),
            );
            let deposit_logs_hash = self.get_deposit_hash(
                deposits
                    .iter()
                    .map(|tx| tx.get_deposit_hash().unwrap())
                    .collect(),
            );

            let state_diff =
                self.prepare_state_diff(&block, store.clone(), withdrawals, deposits)?;

            let blob_commitment = self.prepare_blob_commitment(state_diff)?;
            let mut blob_versioned_hash = keccak(blob_commitment).0;
            blob_versioned_hash[0] = 0x01; // EIP-4844 versioning

            match self
                .send_commitment(
                    block.header.number,
                    H256::from(blob_versioned_hash),
                    withdrawal_logs_merkle_root,
                    deposit_logs_hash,
                )
                .await
            {
                Ok(commit_tx_hash) => {
                    info!(
                    "Sent commitment to block {head_block_hash:#x}, with transaction hash {commit_tx_hash:#x}"
                );
                }
                Err(error) => {
                    error!("Failed to send commitment to block {head_block_hash:#x}. Manual intervention required: {error}");
                    panic!("Failed to send commitment to block {head_block_hash:#x}. Manual intervention required: {error}");
                }
            }

            let proof = Vec::new();

            match self.send_proof(block.header.number, &proof).await {
                Ok(verify_tx_hash) => {
                    info!(
                    "Sent proof for block {head_block_hash}, with transaction hash {verify_tx_hash:#x}"
                );
                }
                Err(error) => {
                    error!("Failed to send proof to block {head_block_hash:#x}. Manual intervention required: {error}");
                    panic!("Failed to send proof to block {head_block_hash:#x}. Manual intervention required: {error}");
                }
            }

            sleep(self.block_production_interval).await;
        }
    }

    pub async fn produce_block(&self, head_block_hash: H256) -> Result<H256, ProposerError> {
        info!("Producing block");
        let fork_choice_state = ForkChoiceState {
            head_block_hash,
            safe_block_hash: head_block_hash,
            finalized_block_hash: head_block_hash,
        };
        let payload_attributes = PayloadAttributesV3 {
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            ..Default::default()
        };
        let fork_choice_response = match self
            .engine_client
            .engine_forkchoice_updated_v3(fork_choice_state, Some(payload_attributes))
            .await
        {
            Ok(response) => response,
            Err(error) => {
                error!("Error sending forkchoiceUpdateV3: {error}");
                return Err(ProposerError::FailedToProduceBlock(format!(
                    "forkchoiceUpdateV3: {error}",
                )));
            }
        };
        let payload_id =
            fork_choice_response
                .payload_id
                .ok_or(ProposerError::FailedToProduceBlock(
                    "payload_id is None in ForkChoiceResponse".to_string(),
                ))?;
        let execution_payload_response =
            match self.engine_client.engine_get_payload_v3(payload_id).await {
                Ok(response) => response,
                Err(error) => {
                    error!("Error sending getPayloadV3: {error}");
                    return Err(ProposerError::FailedToProduceBlock(format!(
                        "getPayloadV3: {error}"
                    )));
                }
            };
        let payload_status = match self
            .engine_client
            .engine_new_payload_v3(
                execution_payload_response.execution_payload,
                Default::default(),
                Default::default(),
            )
            .await
        {
            Ok(response) => response,
            Err(error) => {
                error!("Error sending newPayloadV3: {error}");
                return Err(ProposerError::FailedToProduceBlock(format!(
                    "newPayloadV3: {error}"
                )));
            }
        };
        let produced_block_hash =
            payload_status
                .latest_valid_hash
                .ok_or(ProposerError::FailedToProduceBlock(
                    "latest_valid_hash is None in PayloadStatus".to_string(),
                ))?;
        info!("Produced block {produced_block_hash:#x}");
        Ok(produced_block_hash)
    }

    pub fn get_block_withdrawals(
        &self,
        block: &Block,
    ) -> Result<Vec<(H256, PrivilegedL2Transaction)>, ProposerError> {
        let withdrawals = block
            .body
            .transactions
            .iter()
            .filter_map(|tx| match tx {
                Transaction::PrivilegedL2Transaction(priv_tx)
                    if priv_tx.tx_type == PrivilegedTxType::Withdrawal =>
                {
                    Some((tx.compute_hash(), priv_tx.clone()))
                }
                _ => None,
            })
            .collect();

        Ok(withdrawals)
    }

    pub fn get_withdrawals_merkle_root(&self, withdrawals_hashes: Vec<H256>) -> H256 {
        if !withdrawals_hashes.is_empty() {
            merkelize(withdrawals_hashes)
        } else {
            H256::zero()
        }
    }

    pub fn get_block_deposits(
        &self,
        block: &Block,
    ) -> Result<Vec<PrivilegedL2Transaction>, ProposerError> {
        let deposits = block
            .body
            .transactions
            .iter()
            .filter_map(|tx| match tx {
                Transaction::PrivilegedL2Transaction(tx)
                    if tx.tx_type == PrivilegedTxType::Deposit =>
                {
                    Some(tx.clone())
                }
                _ => None,
            })
            .collect();

        Ok(deposits)
    }

    pub fn get_deposit_hash(&self, deposit_hashes: Vec<H256>) -> H256 {
        if !deposit_hashes.is_empty() {
            H256::from_slice(
                [
                    &(deposit_hashes.len() as u16).to_be_bytes(),
                    &keccak(
                        deposit_hashes
                            .iter()
                            .map(H256::as_bytes)
                            .collect::<Vec<&[u8]>>()
                            .concat(),
                    )
                    .as_bytes()[2..32],
                ]
                .concat()
                .as_slice(),
            )
        } else {
            H256::zero()
        }
    }

    /// Prepare the state diff for the block.
    pub fn prepare_state_diff(
        &self,
        block: &Block,
        store: Store,
        withdrawals: Vec<(H256, PrivilegedL2Transaction)>,
        deposits: Vec<PrivilegedL2Transaction>,
    ) -> Result<StateDiff, ProposerError> {
        info!("Preparing state diff for block {}", block.header.number);

        let mut state = evm_state(store.clone(), block.header.parent_hash);
        execute_block(&block, &mut state).unwrap();
        let account_updates = get_state_transitions(&mut state);

        let mut modified_accounts = HashMap::new();
        account_updates.iter().for_each(|account_update| {
            modified_accounts.insert(
                account_update.address,
                AccountStateDiff {
                    new_balance: account_update.info.clone().map(|info| info.balance),
                    // TODO: Change this with the diff
                    nonce_diff: account_update.info.clone().map(|info| info.nonce as u16),
                    storage: account_update.added_storage.clone().into_iter().collect(),
                    // TODO: Check if bytecode is already known
                    bytecode: account_update.code.clone(),
                    bytecode_hash: None,
                },
            );
        });

        let state_diff = StateDiff {
            modified_accounts,
            version: Default::default(),
            withdrawal_logs: withdrawals
                .iter()
                .map(|(hash, tx)| WithdrawalLog {
                    address: match tx.to {
                        TxKind::Call(address) => address,
                        TxKind::Create => Address::zero(),
                    },
                    amount: tx.value,
                    tx_hash: *hash,
                })
                .collect(),
            deposit_logs: deposits
                .iter()
                .map(|tx| DepositLog {
                    address: match tx.to {
                        TxKind::Call(address) => address,
                        TxKind::Create => Address::zero(),
                    },
                    amount: tx.value,
                })
                .collect(),
        };

        Ok(state_diff)
    }

    /// Prepare the KZG commitment for the blob. This commitment can then be used
    /// to generate the blob versioned hash necessary for the EIP-4844 transaction.
    pub fn prepare_blob_commitment(
        &self,
        state_diff: StateDiff,
    ) -> Result<[u8; 48], ProposerError> {
        let blob_data = state_diff.encode().map_err(ProposerError::from)?;

        let field_elements = blob_data
            .chunks(32)
            .map(|x| {
                if x.len() < 32 {
                    let mut y = [0u8; 32];
                    y[..x.len()].copy_from_slice(x);
                    FrElement::from_bytes_be(&y).unwrap().representative()
                } else {
                    FrElement::from_bytes_be(x).unwrap().representative()
                }
            })
            .collect::<Vec<UnsignedInteger<4>>>();
        if field_elements.len() > 4096 {
            return Err(ProposerError::FailedToProduceBlock(
                "field_elements length is greater than 4096".to_string(),
            ));
        }

        let commitment = BLS12381Curve::compress_g1_point(
            &msm(
                field_elements.iter().as_slice(),
                &self.srs.powers_main_group[..field_elements.len()],
            )
            .expect("`points` is sliced by `cs`'s length")
            .to_affine(),
        );

        Ok(commitment)
    }

    pub async fn send_commitment(
        &self,
        block_number: u64,
        commitment: H256,
        withdrawal_logs_merkle_root: H256,
        deposit_logs_hash: H256,
    ) -> Result<H256, ProposerError> {
        info!("Sending commitment for block {block_number}");
        let mut calldata = Vec::with_capacity(132);
        calldata.extend(COMMIT_FUNCTION_SELECTOR);
        let mut block_number_bytes = [0_u8; 32];
        U256::from(block_number).to_big_endian(&mut block_number_bytes);
        calldata.extend(block_number_bytes);
        calldata.extend(commitment.0);
        calldata.extend(withdrawal_logs_merkle_root.0);
        calldata.extend(deposit_logs_hash.0);

        let commit_tx_hash = self
            .send_transaction_with_calldata(self.on_chain_proposer_address, calldata.into())
            .await?;

        info!("Commitment sent: {commit_tx_hash:#x}");

        while self
            .eth_client
            .get_transaction_receipt(commit_tx_hash)
            .await?
            .is_none()
        {
            sleep(Duration::from_secs(1)).await;
        }

        Ok(commit_tx_hash)
    }

    pub async fn send_proof(
        &self,
        block_number: u64,
        block_proof: &[u8],
    ) -> Result<H256, ProposerError> {
        info!("Sending proof");
        let mut calldata = Vec::new();
        calldata.extend(VERIFY_FUNCTION_SELECTOR);
        let mut block_number_bytes = [0_u8; 32];
        U256::from(block_number).to_big_endian(&mut block_number_bytes);
        calldata.extend(block_number_bytes);
        calldata.extend(H256::from_low_u64_be(32).as_bytes());
        calldata.extend(H256::from_low_u64_be(block_proof.len() as u64).as_bytes());
        calldata.extend(block_proof);
        let leading_zeros = 32 - ((calldata.len() - 4) % 32);
        calldata.extend(vec![0; leading_zeros]);

        let verify_tx_hash = self
            .send_transaction_with_calldata(self.on_chain_proposer_address, calldata.into())
            .await?;

        info!("Proof sent: {verify_tx_hash:#x}");

        while self
            .eth_client
            .get_transaction_receipt(verify_tx_hash)
            .await?
            .is_none()
        {
            sleep(Duration::from_secs(1)).await;
        }

        Ok(verify_tx_hash)
    }

    async fn send_transaction_with_calldata(
        &self,
        to: Address,
        calldata: Bytes,
    ) -> Result<H256, ProposerError> {
        let mut tx = EIP1559Transaction {
            to: TxKind::Call(to),
            data: calldata,
            max_fee_per_gas: self.eth_client.get_gas_price().await?.as_u64(),
            nonce: self.eth_client.get_nonce(self.l1_address).await?,
            chain_id: self.eth_client.get_chain_id().await?.as_u64(),
            ..Default::default()
        };

        let mut generic_tx = GenericTransaction::from(tx.clone());
        generic_tx.from = self.l1_address;

        tx.gas_limit = self
            .eth_client
            .estimate_gas(generic_tx)
            .await?
            .saturating_add(TX_GAS_COST);

        self.eth_client
            .send_eip1559_transaction(&mut tx, self.l1_private_key)
            .await
            .map_err(ProposerError::from)
    }
}
