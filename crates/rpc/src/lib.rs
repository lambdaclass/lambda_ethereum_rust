use axum::{routing::post, Json, Router};
use engine::capabilities::exchange_capabilities;
use eth::{
    block::get_block_by_number,
    client::{chain_id, syncing},
};
use serde_json::{json, Value};
use utils::{RpcErr, RpcErrorResponse, RpcRequest};

mod engine;
mod eth;
mod utils;

#[tokio::main]
pub async fn start_api() {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/", post(handle_request));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8551").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

pub async fn handle_request(body: String) -> Json<Value> {
    let req: RpcRequest = serde_json::from_str(&body).unwrap();

    let res: Result<Value, RpcErr> = match req.method.as_str() {
        "engine_exchangeCapabilities" => exchange_capabilities(),
        "eth_chainId" => chain_id(),
        "eth_syncing" => syncing(),
        "eth_getBlockByNumber" => get_block_by_number(),
        _ => Err(RpcErr::MethodNotFound),
    };

    match res {
        Ok(result) => Json(json!({
            "id": req.id,
            "jsonrpc": "2.0",
            "result": result,
        })),
        Err(error) => {
            let error: RpcErrorResponse = error.into();
            Json(json!({
                "id": req.id,
                "jsonrpc": "2.0",
                "error": error,
            }))
        }
    }
}
