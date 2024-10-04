use ethereum_rust_core::types::EIP1559Transaction;
use ethereum_rust_rlp::encode::RLPEncode;
use ethereum_rust_rpc::{
    types::receipt::RpcLog,
    utils::{RpcErrorResponse, RpcRequest, RpcRequestId, RpcSuccessResponse},
};
use ethereum_types::{Address, H256, U256};
use keccak_hash::keccak;
use libsecp256k1::{sign, Message, SecretKey};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use super::transaction::PayloadRLPEncode;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum RpcResponse {
    Success(RpcSuccessResponse),
    Error(RpcErrorResponse),
}

pub struct L1Rpc {
    client: Client,
    url: String,
}

const EIP1559_TX_TYPE: u8 = 2;

impl L1Rpc {
    pub fn new(url: &str) -> Self {
        Self {
            client: Client::new(),
            url: url.to_string(),
        }
    }

    async fn send_request(&self, request: RpcRequest) -> Result<RpcResponse, reqwest::Error> {
        self.client
            .post(&self.url)
            .header("content-type", "application/json")
            .body(serde_json::ser::to_string(&request).unwrap())
            .send()
            .await?
            .json::<RpcResponse>()
            .await
    }

    pub async fn send_raw_transaction(&self, data: &[u8]) -> Result<H256, String> {
        let request = RpcRequest {
            id: RpcRequestId::Number(1),
            jsonrpc: "2.0".to_string(),
            method: "eth_sendRawTransaction".to_string(),
            params: Some(vec![json!("0x".to_string() + &hex::encode(&data))]),
        };

        match self.send_request(request).await {
            Ok(RpcResponse::Success(result)) => Ok(serde_json::from_value(result.result).unwrap()),
            Ok(RpcResponse::Error(e)) => Err(e.error.message),
            Err(e) => Err(e.to_string()),
        }
    }

    pub async fn send_eip1559_transaction(
        &self,
        mut tx: EIP1559Transaction,
        private_key: SecretKey,
    ) -> Result<H256, String> {
        let mut payload = vec![EIP1559_TX_TYPE];
        payload.append(tx.encode_payload_to_vec().as_mut());

        let data = Message::parse(&keccak(payload).0);
        let signature = sign(&data, &private_key);

        tx.signature_r = U256::from(signature.0.r.b32());
        tx.signature_s = U256::from(signature.0.s.b32());
        tx.signature_y_parity = signature.1.serialize() != 0;

        let mut encoded_tx = Vec::new();
        tx.encode(&mut encoded_tx);

        let mut data = vec![EIP1559_TX_TYPE];
        data.append(&mut encoded_tx);

        self.send_raw_transaction(data.as_slice()).await
    }

    pub async fn get_block_number(&self) -> Result<U256, String> {
        let request = RpcRequest {
            id: RpcRequestId::Number(1),
            jsonrpc: "2.0".to_string(),
            method: "eth_blockNumber".to_string(),
            params: None,
        };

        match self.send_request(request).await {
            Ok(RpcResponse::Success(result)) => Ok(serde_json::from_value(result.result).unwrap()),
            Ok(RpcResponse::Error(e)) => Err(e.error.message),
            Err(e) => Err(e.to_string()),
        }
    }

    pub async fn get_logs(
        &self,
        from_block: U256,
        to_block: U256,
        address: Address,
        topic: H256,
    ) -> Result<Vec<RpcLog>, String> {
        let request = RpcRequest {
            id: RpcRequestId::Number(1),
            jsonrpc: "2.0".to_string(),
            method: "eth_getLogs".to_string(),
            params: Some(vec![serde_json::json!(
                {
                    "fromBlock": format!("{:#x}", from_block),
                    "toBlock": format!("{:#x}", to_block),
                    "address": format!("{:#x}", address),
                    "topics": [format!("{:#x}", topic)]
                }
            )]),
        };

        match self.send_request(request).await {
            Ok(RpcResponse::Success(result)) => Ok(serde_json::from_value(result.result).unwrap()),
            Ok(RpcResponse::Error(e)) => Err(e.error.message),
            Err(e) => Err(e.to_string()),
        }
    }
}
