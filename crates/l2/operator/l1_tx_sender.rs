use crate::utils::{
    config::{eth::EthConfig, l1_tx_sender::L1TxSenderConfig},
    eth_client::EthClient,
};
use ethereum_rust_blockchain::constants::TX_GAS_COST;
use ethereum_rust_core::types::{EIP1559Transaction, TxKind};
use ethereum_types::{Address, H256};
use libsecp256k1::SecretKey;
use tracing::{debug, warn};

const COMMIT_FUNCTION_SELECTOR: [u8; 4] = [227, 206, 9, 77];
const VERIFY_FUNCTION_SELECTOR: [u8; 4] = [142, 118, 10, 254];

pub struct L1TxSender {
    rpc_url: String,
    contract_address: Address,
    operator_address: Address,
    operator_private_key: SecretKey,
}

impl L1TxSender {
    pub fn new_from_config(sender_config: L1TxSenderConfig, eth_config: EthConfig) -> Self {
        Self {
            rpc_url: eth_config.rpc_url,
            contract_address: sender_config.block_executor_address,
            operator_address: sender_config.operator_address,
            operator_private_key: sender_config.operator_private_key,
        }
    }

    async fn send_transaction(&self, mut tx: EIP1559Transaction) -> Result<H256, String> {
        let client = EthClient::new(&self.rpc_url);

        tx.gas_limit = client
            .estimate_gas(tx.clone())
            .await?
            .saturating_add(TX_GAS_COST);

        tx.max_fee_per_gas = client.get_gas_price().await?;

        tx.nonce = client.get_nonce(self.operator_address).await?;

        client
            .send_eip1559_transaction(tx, self.operator_private_key)
            .await
    }

    pub async fn send_commitment(
        &self,
        previous_commitment: H256,
        current_commitment: H256,
    ) -> Result<H256, String> {
        let mut calldata = Vec::with_capacity(68);
        calldata.extend(COMMIT_FUNCTION_SELECTOR);
        calldata.extend(previous_commitment.0);
        calldata.extend(current_commitment.0);

        let tx = EIP1559Transaction {
            to: TxKind::Call(self.contract_address),
            data: calldata.into(),
            chain_id: 3151908,
            ..Default::default()
        };

        match self.send_transaction(tx).await {
            Ok(hash) => {
                debug!("Commitment sent: {:#?}", hash);
                Ok(hash)
            }
            Err(e) => {
                warn!("Failed to send commitment: {:#?}", e);
                Err(e)
            }
        }
    }

    pub async fn send_verification(&self, block_proof: &[u8]) -> Result<H256, String> {
        let mut calldata = Vec::new();
        calldata.extend(VERIFY_FUNCTION_SELECTOR);
        calldata.extend(H256::from_low_u64_be(32).as_bytes());
        calldata.extend(H256::from_low_u64_be(block_proof.len() as u64).as_bytes());
        calldata.extend(block_proof);
        let leading_zeros = 32 - (calldata.len() % 32);
        calldata.extend(vec![0; leading_zeros]);

        let tx = EIP1559Transaction {
            to: TxKind::Call(self.contract_address),
            data: calldata.into(),
            chain_id: 3151908,
            ..Default::default()
        };

        match self.send_transaction(tx).await {
            Ok(hash) => {
                debug!("Verification sent: {:#?}", hash);
                Ok(hash)
            }
            Err(e) => {
                warn!("Failed to send verification: {:#?}", e);
                Err(e)
            }
        }
    }
}

pub async fn start_l1_tx_sender() {}
