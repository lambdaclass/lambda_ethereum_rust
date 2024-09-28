use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use ethereum_rust_rpc::utils::{RpcErrorResponse, RpcRequest, RpcRequestId, RpcSuccessResponse};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum RpcResponse {
    Success(RpcSuccessResponse),
    Error(RpcErrorResponse),
}

pub struct ConsensusMock {
    client: Client,
    secret: Bytes,
    execution_client_url: String,
}

impl ConsensusMock {
    pub fn new(execution_client_url: &str) -> Self {
        Self {
            client: Client::new(),
            secret: Bytes::from_static(include_bytes!("../jwt.hex")),
            execution_client_url: execution_client_url.to_string(),
        }
    }

    async fn send_request(&self, request: RpcRequest) -> Result<RpcResponse, reqwest::Error> {
        self.client
            .post(&self.execution_client_url)
            .bearer_auth(self.auth_token())
            .header("content-type", "application/json")
            .body(serde_json::ser::to_string(&request).unwrap())
            .send()
            .await?
            .json::<RpcResponse>()
            .await
    }

    pub async fn engine_exchange_capabilities(&self) -> Result<Vec<String>, String> {
        let request = RpcRequest {
            id: RpcRequestId::Number(1),
            jsonrpc: "2.0".to_string(),
            method: "engine_exchangeCapabilities".to_string(),
            params: Some(vec![serde_json::to_value(Self::capabilities()).unwrap()]),
        };

        match self.send_request(request).await {
            Ok(RpcResponse::Success(result)) => Ok(serde_json::from_value(result.result).unwrap()),
            Ok(RpcResponse::Error(e)) => Err(e.error.message),
            Err(e) => Err(e.to_string()),
        }
    }

    pub async fn engine_forkchoice_updated_v3(&self) -> Result<(), String> {
        let request = RpcRequest {
            id: RpcRequestId::Number(1),
            jsonrpc: "2.0".to_string(),
            method: "engine_forkchoiceUpdatedV3".to_string(),
            params: None,
        };

        match self.send_request(request).await {
            Ok(RpcResponse::Success(_)) => Ok(()),
            Ok(RpcResponse::Error(e)) => Err(e.error.message),
            Err(e) => Err(e.to_string()),
        }
    }

    fn auth_token(&self) -> String {
        // Header
        let header = jsonwebtoken::Header::default();
        // Claims
        let valid_iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        let claims = json!({"iat": valid_iat});
        // Encoding Key
        let decoded_secret = hex::decode(self.secret.clone()).unwrap();
        let encoding_key = jsonwebtoken::EncodingKey::from_secret(decoded_secret.as_ref());
        // JWT Token
        jsonwebtoken::encode(&header, &claims, &encoding_key).unwrap()
    }

    fn capabilities() -> Vec<String> {
        vec!["engine_exchangeCapabilities".to_owned()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_exchange_capabilities() {
        let consensus_mock_client = ConsensusMock::new("http://localhost:8551");

        println!(
            "{:?}",
            consensus_mock_client
                .engine_exchange_capabilities()
                .await
                .unwrap()
        );
    }
}
