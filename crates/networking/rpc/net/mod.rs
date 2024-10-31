use serde_json::Value;

use crate::{
    utils::{RpcErr, RpcRequest},
    RpcApiContext,
};

pub fn version(_req: &RpcRequest, context: RpcApiContext) -> Result<Value, RpcErr> {
    let chain_spec = context
        .storage
        .get_chain_config()
        .map_err(|error| RpcErr::Internal(error.to_string()))?;
    serde_json::to_value(format!("{}", chain_spec.chain_id))
        .map_err(|error| RpcErr::Internal(error.to_string()))
}
