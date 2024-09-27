use ethereum_rust_rpc::{
    types::receipt::RpcLog,
    utils::{RpcErrorResponse, RpcSuccessResponse},
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

    async fn send_request(
        &self,
        method: &str,
        params: Option<&str>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .post(&self.url)
            .header("content-type", "application/json")
            .body(
                r#"{"jsonrpc":"2.0","method":""#.to_string()
                    + method
                    + r#"","params":"#
                    + params.unwrap_or("[]")
                    + r#","id":1}"#,
            )
            .send()
            .await
    }

    pub async fn get_block_number(&self) -> Result<U256, String> {
        match self.send_request("eth_blockNumber", None).await {
            Ok(res) => match res.json::<RpcResponse>().await {
                Ok(body) => match body {
                    RpcResponse::Success(result) => {
                        Ok(serde_json::from_value(result.result).unwrap())
                    }
                    RpcResponse::Error(error) => Err(error.error.message),
                },
                Err(e) => Err(e.to_string()),
            },
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
        let params = format!(
            r#"[{{"fromBlock": "{:#x}", "toBlock": "{:#x}", "address": "{:#x}", "topics": ["{:#x}"]}}]"#,
            from_block, to_block, address, topic
        );

        match self.send_request("eth_getLogs", Some(&params)).await {
            Ok(res) => match res.json::<RpcResponse>().await {
                Ok(body) => match body {
                    RpcResponse::Success(result) => {
                        Ok(serde_json::from_value(result.result).unwrap())
                    }
                    RpcResponse::Error(error) => Err(error.error.message),
                },
                Err(e) => Err(e.to_string()),
            },
            Err(e) => Err(e.to_string()),
        }
    }
}
