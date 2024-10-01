use bytes::Bytes;
use ethereum_rust_rpc::{
    types::{
        fork_choice::{ForkChoiceResponse, ForkChoiceState, PayloadAttributesV3},
        payload::{ExecutionPayloadResponse, ExecutionPayloadV3, PayloadStatus},
    },
    utils::{RpcErrorResponse, RpcRequest, RpcRequestId, RpcSuccessResponse},
};
use ethereum_types::{H256, U256};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

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
    pub fn new(execution_client_url: &str, secret: Bytes) -> Self {
        Self {
            client: Client::new(),
            secret,
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

    pub async fn engine_forkchoice_updated_v3(&self) -> Result<ForkChoiceResponse, String> {
        let genesis_block_hash =
            H256::from_str("0x72cb6312947af2b38ec764b9932087edc7eab201e5025afd5d4bfe3172b3648b")
                .unwrap();
        let forkchoice_state = ForkChoiceState {
            head_block_hash: genesis_block_hash,
            safe_block_hash: genesis_block_hash,
            finalized_block_hash: genesis_block_hash,
        };
        let payload_attributes_v3 = PayloadAttributesV3::default();
        let request = RpcRequest {
            id: RpcRequestId::Number(1),
            jsonrpc: "2.0".to_string(),
            method: "engine_forkchoiceUpdatedV3".to_string(),
            params: Some(vec![
                serde_json::to_value(forkchoice_state).unwrap(),
                serde_json::to_value(payload_attributes_v3).unwrap(),
            ]),
        };

        match self.send_request(request).await {
            Ok(RpcResponse::Success(s)) => Ok(serde_json::from_value(s.result).unwrap()),
            Ok(RpcResponse::Error(e)) => Err(e.error.message),
            Err(e) => Err(e.to_string()),
        }
    }

    pub async fn engine_get_payload_v3(
        &self,
        payload_id: u64,
    ) -> Result<ExecutionPayloadResponse, String> {
        let request = RpcRequest {
            id: RpcRequestId::Number(1),
            jsonrpc: "2.0".to_string(),
            method: "engine_getPayloadV3".to_string(),
            params: Some(vec![json!(U256::from(payload_id))]),
        };

        match self.send_request(request).await {
            Ok(RpcResponse::Success(s)) => Ok(serde_json::from_value(s.result).unwrap()),
            Ok(RpcResponse::Error(e)) => Err(e.error.message),
            Err(e) => Err(e.to_string()),
        }
    }

    pub async fn engine_new_payload_v3(
        &self,
        execution_payload: ExecutionPayloadV3,
        expected_blob_versioned_hashes: Vec<H256>,
        parent_beacon_block_root: H256,
    ) -> Result<PayloadStatus, String> {
        let request = RpcRequest {
            id: RpcRequestId::Number(1),
            jsonrpc: "2.0".to_string(),
            method: "engine_newPayloadV3".to_string(),
            params: Some(vec![
                serde_json::to_value(execution_payload).unwrap(),
                serde_json::to_value(expected_blob_versioned_hashes).unwrap(),
                serde_json::to_value(parent_beacon_block_root).unwrap(),
            ]),
        };

        match self.send_request(request).await {
            Ok(RpcResponse::Success(s)) => Ok(serde_json::from_value(s.result).unwrap()),
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
    async fn test_add_block() {
        let secret = Bytes::from_static(include_bytes!(
            "/Users/ivanlitteri/Repositories/lambdaclass/ethereum_rust/jwt.hex"
        ));
        let consensus_mock_client = ConsensusMock::new("http://localhost:8551", secret);

        let fork_choice_response = consensus_mock_client
            .engine_forkchoice_updated_v3()
            .await
            .unwrap();

        println!("{fork_choice_response:#?}\n");

        let payload_id = fork_choice_response.payload_id.unwrap();

        println!("PAYLOAD ID: {payload_id:#?}\n");

        let execution_payload_response = consensus_mock_client
            .engine_get_payload_v3(payload_id)
            .await
            .unwrap();

        println!("{execution_payload_response:#?}\n");

        let payload_status = consensus_mock_client
            .engine_new_payload_v3(
                execution_payload_response.execution_payload,
                Default::default(),
                Default::default(),
            )
            .await
            .unwrap();

        println!("{payload_status:#?}");
    }
}
