use std::str::FromStr;

use ethereum_rust_core::types::EIP1559Transaction;
use ethereum_rust_rlp::{encode::RLPEncode, structs::Encoder};
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

    pub async fn send_raw_transaction(&self, tx_type: u8, data: &[u8]) -> Result<H256, String> {
        let request = RpcRequest {
            id: RpcRequestId::Number(1),
            jsonrpc: "2.0".to_string(),
            method: "eth_sendRawTransaction".to_string(),
            params: Some(vec![json!(
                "0x".to_string() + &hex::encode([tx_type]) + &hex::encode(&data)
            )]),
        };

        match self
            .send_request(request)
            .await
            .map_err(|e| e.to_string())?
        {
            RpcResponse::Success(response) => match response.result.as_str() {
                Some(result) => Ok(H256::from_str(&result).unwrap()),
                None => Err("Unexpected response format".to_string()),
            },
            RpcResponse::Error(error) => Err(error.error.message),
        }
    }

    fn rlp_encode_payload(tx: &EIP1559Transaction) -> Vec<u8> {
        let mut buf = Vec::new();
        Encoder::new(&mut buf)
            .encode_field(&tx.chain_id)
            .encode_field(&tx.nonce)
            .encode_field(&tx.max_priority_fee_per_gas)
            .encode_field(&tx.max_fee_per_gas)
            .encode_field(&tx.gas_limit)
            .encode_field(&tx.to)
            .encode_field(&tx.value)
            .encode_field(&tx.data)
            .encode_field(&tx.access_list)
            .finish();
        buf
    }

    pub async fn send_eip1559_transaction(
        &self,
        tx: EIP1559Transaction,
        private_key: SecretKey,
    ) -> Result<H256, String> {
        let mut tx = tx.clone();
        let mut payload = vec![EIP1559_TX_TYPE];
        payload.append(&mut Self::rlp_encode_payload(&tx));

        let data = Message::parse(&keccak(payload).0);
        let signature = sign(&data, &private_key);

        tx.signature_r = U256::from(signature.0.r.b32());
        tx.signature_s = U256::from(signature.0.s.b32());
        tx.signature_y_parity = signature.1.serialize() != 0;

        let mut encoded_tx = Vec::new();
        tx.encode(&mut encoded_tx);

        self.send_raw_transaction(EIP1559_TX_TYPE, encoded_tx.as_slice())
            .await
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
