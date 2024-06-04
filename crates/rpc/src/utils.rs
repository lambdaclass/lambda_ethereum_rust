use serde::{Deserialize, Serialize};
use serde_json::Value;

pub enum RpcErr {
    MethodNotFound,
}

impl Into<RpcErrorResponse> for RpcErr {
    fn into(self) -> RpcErrorResponse {
        match self {
            RpcErr::MethodNotFound => RpcErrorResponse {
                code: -32601,
                message: "Method not found".to_string(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcRequest {
    pub id: i32,
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcErrorResponse {
    pub code: i32,
    pub message: String,
}
