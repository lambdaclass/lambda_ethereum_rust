use crate::utils::eth_client::EthClient;
use ethereum_rust_blockchain::constants::TX_GAS_COST;
use ethereum_rust_core::types::{EIP1559Transaction, TxKind};
use ethereum_types::{Address, H256};
use libsecp256k1::SecretKey;
use std::str::FromStr;
use tracing::{debug, warn};

const RICH_WALLET_PK: &str = "0x385c546456b6a603a1cfcaa9ec9494ba4832da08dd6bcf4de9a71e4a01b74924";
const RICH_WALLET_ADDR: &str = "0x3D1e15a1a55578f7c920884a9943b3B35D0D885b";
const BLOCK_EXECUTOR_ADDR: &str = "0x31e68fE377E509c8324b6206ADC7f11003Bd9130";
const COMMIT_FUNCTION_SELECTOR: [u8; 4] = [227, 206, 9, 77];
const VERIFY_FUNCTION_SELECTOR: [u8; 4] = [142, 118, 10, 254];

pub struct L1TxSender;

impl L1TxSender {
    async fn send_transaction(mut tx: EIP1559Transaction) -> Result<H256, String> {
        let client = EthClient::new("http://localhost:8545");
        let private_key = SecretKey::parse(&H256::from_str(RICH_WALLET_PK).unwrap().0).unwrap();

        tx.gas_limit = client
            .estimate_gas(tx.clone())
            .await?
            .saturating_add(TX_GAS_COST);

        tx.max_fee_per_gas = client.get_gas_price().await?;

        tx.nonce = client
            .get_nonce(Address::from_str(&RICH_WALLET_ADDR[2..]).unwrap())
            .await?;

        client.send_eip1559_transaction(tx, private_key).await
    }

    pub async fn send_commitment(
        previous_commitment: H256,
        current_commitment: H256,
    ) -> Result<H256, String> {
        let mut calldata = Vec::with_capacity(68);
        calldata.extend(COMMIT_FUNCTION_SELECTOR);
        calldata.extend(previous_commitment.0);
        calldata.extend(current_commitment.0);

        let tx = EIP1559Transaction {
            to: TxKind::Call(Address::from_str(BLOCK_EXECUTOR_ADDR).unwrap()),
            data: calldata.into(),
            chain_id: 3151908,
            ..Default::default()
        };

        match Self::send_transaction(tx).await {
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

    pub async fn send_verification(block_proof: &[u8]) -> Result<H256, String> {
        let mut calldata = Vec::new();
        calldata.extend(VERIFY_FUNCTION_SELECTOR);
        calldata.extend(H256::from_low_u64_be(32).as_bytes());
        calldata.extend(H256::from_low_u64_be(block_proof.len() as u64).as_bytes());
        calldata.extend(block_proof);
        let leading_zeros = 32 - (calldata.len() % 32);
        calldata.extend(vec![0; leading_zeros]);

        let tx = EIP1559Transaction {
            to: TxKind::Call(Address::from_str(BLOCK_EXECUTOR_ADDR).unwrap()),
            data: calldata.into(),
            chain_id: 3151908,
            ..Default::default()
        };

        match Self::send_transaction(tx).await {
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
