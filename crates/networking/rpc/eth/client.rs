use tracing::info;

use ethereum_rust_storage::Store;
use serde_json::Value;

use crate::{utils::RpcErr, RpcHandler};

pub struct ChainId;
impl RpcHandler for ChainId {
    fn parse(_params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        Ok(Self {})
    }

    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        info!("Requested chain id");
        let chain_spec = storage.get_chain_config().map_err(|_| RpcErr::Internal)?;
        serde_json::to_value(format!("{:#x}", chain_spec.chain_id)).map_err(|_| RpcErr::Internal)
    }
}

pub struct Syncing;
impl RpcHandler for Syncing {
    fn parse(_params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        Ok(Self {})
    }

    fn handle(&self, _storage: Store) -> Result<Value, RpcErr> {
        Ok(Value::Bool(false))
    }
}
