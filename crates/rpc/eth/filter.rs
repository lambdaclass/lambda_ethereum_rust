use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::block_identifier::BlockIdentifier;
use crate::utils::RpcErr;
use crate::RpcHandler;
use ethereum_rust_core::types::LogsFilter;
use rand::prelude::*;

use super::logs::LogsRequest;

#[derive(Debug, Clone)]
pub struct FilterRequest {
    pub request_data: LogsRequest,
}

impl RpcHandler for FilterRequest {
    fn parse(params: &Option<Vec<serde_json::Value>>) -> Result<Self, crate::utils::RpcErr> {
        let filter = LogsRequest::parse(params)?;
        Ok(FilterRequest {
            request_data: filter,
        })
    }
    fn handle(
        &self,
        storage: ethereum_rust_storage::Store,
    ) -> Result<serde_json::Value, crate::utils::RpcErr> {
        let filter = self.request_data.request_to_filter(&storage)?;
        let id: u64 = random();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|unix_time| unix_time.as_secs())
            .map_err(|_err| RpcErr::Internal)?;

        storage.add_filter(random(), timestamp, filter)?;
        let as_hex = format!("{:x}", id);
        return Ok(as_hex.into());
    }
}
