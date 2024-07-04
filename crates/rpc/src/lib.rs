use std::{future::IntoFuture, net::SocketAddr};

use axum::{routing::post, Json, Router};
use engine::{ExchangeCapabilitiesRequest, NewPayloadV3Request};
use eth::{block, client};
use serde_json::Value;
use tokio::net::TcpListener;
use tracing::info;
use utils::{RpcErr, RpcErrorMetadata, RpcErrorResponse, RpcRequest, RpcSuccessResponse};

mod admin;
mod engine;
mod eth;
mod utils;

pub async fn start_api(http_addr: SocketAddr, authrpc_addr: SocketAddr) {
    let http_router = Router::new().route("/", post(handle_http_request));
    let http_listener = TcpListener::bind(http_addr).await.unwrap();

    let authrpc_router = Router::new().route("/", post(handle_authrpc_request));
    let authrpc_listener = TcpListener::bind(authrpc_addr).await.unwrap();

    let authrpc_server = axum::serve(authrpc_listener, authrpc_router)
        .with_graceful_shutdown(shutdown_signal())
        .into_future();
    let http_server = axum::serve(http_listener, http_router)
        .with_graceful_shutdown(shutdown_signal())
        .into_future();

    info!("Starting HTTP server at {http_addr}");
    info!("Starting Auth-RPC server at {}", authrpc_addr);

    let _ = tokio::try_join!(authrpc_server, http_server)
        .inspect_err(|e| info!("Error shutting down servers: {:?}", e));
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
}

pub async fn handle_authrpc_request(body: String) -> Json<Value> {
    let req: RpcRequest = serde_json::from_str(&body).unwrap();
    let res = map_requests(&req);
    rpc_response(req.id, res)
}

pub fn map_requests(req: &RpcRequest) -> Result<Value, RpcErr> {
    match req.method.as_str() {
        "engine_exchangeCapabilities" => {
            let capabilities: ExchangeCapabilitiesRequest = req
                .params
                .as_ref()
                .ok_or(RpcErr::BadParams)?
                .first()
                .ok_or(RpcErr::BadParams)
                .and_then(|v| serde_json::from_value(v.clone()).map_err(|_| RpcErr::BadParams))?;
            engine::exchange_capabilities(&capabilities)
        }
        "eth_chainId" => client::chain_id(),
        "eth_syncing" => client::syncing(),
        "eth_getBlockByNumber" => block::get_block_by_number(),
        "engine_forkchoiceUpdatedV3" => engine::forkchoice_updated_v3(),
        "engine_newPayloadV3" => {
            let request =
                parse_new_payload_v3_request(req.params.as_ref().ok_or(RpcErr::BadParams)?)?;
            Ok(serde_json::to_value(engine::new_payload_v3(request)?).unwrap())
        }
        _ => Err(RpcErr::MethodNotFound),
    }
}

pub async fn handle_http_request(body: String) -> Json<Value> {
    let req: RpcRequest = serde_json::from_str(&body).unwrap();

    let res: Result<Value, RpcErr> = match req.method.as_str() {
        "eth_chainId" => client::chain_id(),
        "eth_syncing" => client::syncing(),
        "eth_getBlockByNumber" => block::get_block_by_number(),
        "admin_nodeInfo" => admin::node_info(),
        _ => Err(RpcErr::MethodNotFound),
    };

    rpc_response(req.id, res)
}

fn rpc_response<E>(id: i32, res: Result<Value, E>) -> Json<Value>
where
    E: Into<RpcErrorMetadata>,
{
    match res {
        Ok(result) => Json(
            serde_json::to_value(RpcSuccessResponse {
                id,
                jsonrpc: "2.0".to_string(),
                result,
            })
            .unwrap(),
        ),
        Err(error) => Json(
            serde_json::to_value(RpcErrorResponse {
                id,
                jsonrpc: "2.0".to_string(),
                error: error.into(),
            })
            .unwrap(),
        ),
    }
}

fn parse_new_payload_v3_request(params: &Vec<Value>) -> Result<NewPayloadV3Request, RpcErr> {
    if params.len() != 3 {
        return Err(RpcErr::BadParams);
    }
    let payload = serde_json::from_value(params[0].clone()).map_err(|_| RpcErr::BadParams)?;
    let expected_blob_versioned_hashes =
        serde_json::from_value(params[1].clone()).map_err(|_| RpcErr::BadParams)?;
    let parent_beacon_block_root =
        serde_json::from_value(params[2].clone()).map_err(|_| RpcErr::BadParams)?;
    Ok(NewPayloadV3Request {
        payload,
        expected_blob_versioned_hashes,
        parent_beacon_block_root,
    })
}
