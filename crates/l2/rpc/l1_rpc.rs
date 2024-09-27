use ethereum_rust_rpc::{
    types::receipt::RpcLog,
    utils::{RpcErrorResponse, RpcRequest, RpcRequestId, RpcSuccessResponse},
};
use ethereum_types::{Address, H256, U256};
use reqwest::Client;
use serde::Deserialize;

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
