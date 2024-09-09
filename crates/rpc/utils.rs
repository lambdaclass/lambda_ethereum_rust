use ethereum_rust_evm::EvmError;
use ethereum_rust_storage::error::StoreError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::authentication::AuthenticationError;

#[derive(Debug)]
pub enum RpcErr {
    MethodNotFound,
    BadParams,
    UnsuportedFork,
    Internal,
    Vm,
    Revert { data: String },
    AuthenticationError(AuthenticationError),
}

impl From<RpcErr> for RpcErrorMetadata {
    fn from(value: RpcErr) -> Self {
        match value {
            RpcErr::MethodNotFound => RpcErrorMetadata {
                code: -32601,
                data: None,
                message: "Method not found".to_string(),
            },
            RpcErr::BadParams => RpcErrorMetadata {
                code: -32000,
                data: None,
                message: "Invalid params".to_string(),
            },
            RpcErr::UnsuportedFork => RpcErrorMetadata {
                code: -38005,
                data: None,
                message: "Unsupported fork".to_string(),
            },
            RpcErr::Internal => RpcErrorMetadata {
                code: -32603,
                data: None,
                message: "Internal Error".to_string(),
            },
            RpcErr::Vm => RpcErrorMetadata {
                code: -32015,
                data: None,
                message: "Vm execution error".to_string(),
            },
            RpcErr::Revert { data } => RpcErrorMetadata {
                // This code (3) was hand-picked to match hive tests.
                // Could not find proper documentation about it.
                code: 3,
                data: Some(data.clone()),
                message: format!(
                    "execution reverted: {}",
                    get_message_from_revert_data(&data)
                ),
            },
            RpcErr::AuthenticationError(auth_error) => match auth_error {
                AuthenticationError::InvalidIssuedAtClaim => RpcErrorMetadata {
                    code: -32000,
                    data: None,
                    message: "Auth failed: Invalid iat claim".to_string(),
                },
                AuthenticationError::TokenDecodingError => RpcErrorMetadata {
                    code: -32000,
                    data: None,
                    message: "Auth failed: Invalid or missing token".to_string(),
                },
                AuthenticationError::MissingAuthentication => RpcErrorMetadata {
                    code: -32000,
                    data: None,
                    message: "Auth failed: Missing authentication header".to_string(),
                },
            },
        }
    }
}

pub enum RpcNamespace {
    Engine,
    Eth,
    Admin,
    Debug,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcRequest {
    pub id: i32,
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Vec<Value>>,
}

impl RpcRequest {
    pub fn namespace(&self) -> Result<RpcNamespace, RpcErr> {
        let mut parts = self.method.split('_');
        if let Some(namespace) = parts.next() {
            match namespace {
                "engine" => Ok(RpcNamespace::Engine),
                "eth" => Ok(RpcNamespace::Eth),
                "admin" => Ok(RpcNamespace::Admin),
                "debug" => Ok(RpcNamespace::Debug),
                _ => Err(RpcErr::MethodNotFound),
            }
        } else {
            Err(RpcErr::MethodNotFound)
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcErrorMetadata {
    pub code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcSuccessResponse {
    pub id: i32,
    pub jsonrpc: String,
    pub result: Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcErrorResponse {
    pub id: i32,
    pub jsonrpc: String,
    pub error: RpcErrorMetadata,
}

/// Failure to read from DB will always constitute an internal error
impl From<StoreError> for RpcErr {
    fn from(_value: StoreError) -> Self {
        RpcErr::Internal
    }
}

impl From<EvmError> for RpcErr {
    fn from(_value: EvmError) -> Self {
        RpcErr::Vm
    }
}

fn get_message_from_revert_data(_data: &str) -> String {
    // TODO
    // Hive tests are not failing when revert message does not match, but currently it is not matching
    // It should be fixed
    // See https://github.com/ethereum/go-ethereum/blob/8fd43c80132434dca896d8ae5004ae2aac1450d3/accounts/abi/abi.go#L275
    "".to_owned()
}
