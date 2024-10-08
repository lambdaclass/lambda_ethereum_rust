use crate::utils::eth_client::{transaction::PayloadRLPEncode, EthClient};
use bytes::Bytes;
use ethereum_rust_blockchain::{constants::TX_GAS_COST, mempool};
use ethereum_rust_core::types::{EIP1559Transaction, Transaction, TxKind, TxType};
use ethereum_rust_rlp::encode::RLPEncode;
use ethereum_rust_rpc::types::receipt::RpcLog;
use ethereum_rust_storage::Store;
use ethereum_types::{Address, BigEndianHash, H256, U256};
use keccak_hash::keccak;
use libsecp256k1::{sign, Message, SecretKey};
use std::{cmp::min, ops::Mul, str::FromStr, time::Duration};
use tokio::time::sleep;
use tracing::{debug, info, warn};

pub async fn start_l1_watcher(store: Store) {
    // This address and topic were used for testing purposes only.
    // TODO: Receive them as parameters from config.
    let mut l1_watcher = L1Watcher::new(
        "0x90cD151b6e9500F13240dF68673f3B81245D57d4"
            .parse()
            .unwrap(),
        vec![
            "0x6f65d68a35457dd88c1f8641be5da191aa122bc76de22ab0789dcc71929d7d37"
                .parse()
                .unwrap(),
        ],
    );
    loop {
        let logs = l1_watcher.get_logs().await;
        l1_watcher.process_logs(logs, &store).await;
    }
}

pub struct L1Watcher {
    eth_client: EthClient,
    address: Address,
    topics: Vec<H256>,
    last_block_fetched: U256,
}

impl L1Watcher {
    pub fn new(address: Address, topics: Vec<H256>) -> Self {
        let l1_rpc = EthClient::new("http://localhost:8545");
        Self {
            eth_client: l1_rpc,
            address,
            topics,
            last_block_fetched: U256::zero(),
        }
    }

    pub async fn get_logs(&mut self) -> Vec<RpcLog> {
        let step = U256::from(5000);

        let current_block = self.eth_client.get_block_number().await.unwrap();

        debug!(
            "Current block number: {} ({:#x})",
            current_block, current_block
        );

        let new_last_block = min(self.last_block_fetched + step, current_block);

        debug!(
            "Looking logs from block {:#x} to {:#x}",
            self.last_block_fetched, new_last_block
        );

        let logs = self
            .eth_client
            .get_logs(
                self.last_block_fetched,
                new_last_block,
                self.address,
                self.topics[0],
            )
            .await
            .unwrap();

        debug!("Logs: {:#?}", logs);

        self.last_block_fetched = new_last_block;

        sleep(Duration::from_secs(5)).await;

        logs
    }

    pub async fn process_logs(&self, logs: Vec<RpcLog>, store: &Store) {
        for log in logs {
            let mint_value = format!("{:#x}", log.log.topics[1]).parse::<U256>().unwrap();
            let beneficiary = format!("{:#x}", log.log.topics[2].into_uint())
                .parse::<Address>()
                .unwrap();

            info!(
                "Initiating mint transaction for {:#x} with value {:#x}",
                beneficiary, mint_value
            );

            let mut mint_transaction = EIP1559Transaction {
                to: TxKind::Call(beneficiary),
                data: Bytes::from(b"mint".as_slice()),
                chain_id: store.get_chain_config().unwrap().chain_id,
                ..Default::default()
            };

            let private_key = SecretKey::parse(
                &H256::from_str(
                    "0x385c546456b6a603a1cfcaa9ec9494ba4832da08dd6bcf4de9a71e4a01b74924",
                )
                .unwrap()
                .0,
            )
            .unwrap();

            mint_transaction.nonce = store
                .get_account_info(
                    self.eth_client.get_block_number().await.unwrap().as_u64(),
                    beneficiary,
                )
                .unwrap()
                .map(|info| info.nonce)
                .unwrap_or_default();
            mint_transaction.max_fee_per_gas = self.eth_client.gas_price().await.unwrap().as_u64();
            // TODO(IMPORTANT): gas_limit should come in the log and must
            // not be calculated in here. The reason for this is that the
            // gas_limit for this transaction is payed by the caller in
            // the L1 as part of the deposited funds.
            mint_transaction.gas_limit = TX_GAS_COST.mul(2);
            mint_transaction.value = mint_value;

            let mut payload = vec![TxType::EIP1559 as u8];
            payload.append(mint_transaction.encode_payload_to_vec().as_mut());

            let data = Message::parse(&keccak(payload).0);
            let signature = sign(&data, &private_key);

            mint_transaction.signature_r = U256::from(signature.0.r.b32());
            mint_transaction.signature_s = U256::from(signature.0.s.b32());
            mint_transaction.signature_y_parity = signature.1.serialize() != 0;

            let mut encoded_tx = Vec::new();
            mint_transaction.encode(&mut encoded_tx);

            let mut data = vec![TxType::EIP1559 as u8];
            data.append(&mut encoded_tx);

            match mempool::add_transaction(
                Transaction::EIP1559Transaction(mint_transaction),
                store.clone(),
            ) {
                Ok(hash) => {
                    info!("Mint transaction added to mempool {hash:#x}",);
                    // Ok(hash)
                }
                Err(e) => {
                    warn!("Failed to add mint transaction to the mempool: {e:#?}");
                    // Err(e)
                }
            }
        }
    }
}
