use serde::{Deserialize, Serialize};
use serde_json::Value;

pub enum RpcErr {
    MethodNotFound,
    BadParams,
    UnsuportedFork,
}

impl From<RpcErr> for RpcErrorMetadata {
    fn from(value: RpcErr) -> Self {
        match value {
            RpcErr::MethodNotFound => RpcErrorMetadata {
                code: -32601,
                message: "Method not found".to_string(),
            },
            RpcErr::BadParams => RpcErrorMetadata {
                code: -32602,
                message: "Invalid params".to_string(),
            },
            RpcErr::UnsuportedFork => RpcErrorMetadata {
                code: -38005,
                message: "Unsupported fork".to_string(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcRequest {
    pub id: i32,
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Vec<Value>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcErrorMetadata {
    pub code: i32,
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
