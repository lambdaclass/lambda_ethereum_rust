use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::RpcHandler;

use super::logs::LogsRequest;

#[derive(Debug, Clone)]
pub struct FilterRequest {
    /// Timestamp for when the filter was registered,
    /// it will last 5 minutes.
    pub timestamp: SystemTime,
    // TODO: Move this to a proper type
    pub filter: LogsRequest,
}

impl RpcHandler for Filter {
    fn parse(params: &Option<Vec<serde_json::Value>>) -> Result<Self, crate::utils::RpcErr> {
        let filter = LogsRequest::parse(params)?;
        let now = std::time::SystemTime::now();
        Ok(FilterRequest {
            filter,
            timestamp: now,
        })
    }
    fn handle(
        &self,
        storage: ethereum_rust_storage::Store,
    ) -> Result<serde_json::Value, crate::utils::RpcErr> {
        let LogsRequest {
            from_block,
            to_block,
            address: address_filters,
            topics,
        } = self.filter;
        storage.add_filter(
            self.timestamp,
            from_block,
            to_block,
            address_filters,
            topics,
        )
    }
}
