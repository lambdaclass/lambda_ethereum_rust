use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::RpcHandler;

#[derive(Debug, Clone)]
pub struct Filter {
    /// Timestamp for when the filter was registered,
    /// it will last 5 minutes.
    pub timestamp: u64,
}

impl RpcHandler for Filter {
    fn parse(params: &Option<Vec<serde_json::Value>>) -> Result<Self, crate::utils::RpcErr> {
        todo!()
    }
    fn call(
        req: &crate::utils::RpcRequest,
        storage: ethereum_rust_storage::Store,
    ) -> Result<serde_json::Value, crate::utils::RpcErr> {
        todo!()
    }
    fn handle(
        &self,
        storage: ethereum_rust_storage::Store,
    ) -> Result<serde_json::Value, crate::utils::RpcErr> {
        todo!()
    }
}
