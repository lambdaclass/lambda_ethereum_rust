use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use tracing::error;

use crate::utils::{RpcErr, RpcRequest};
use crate::RpcHandler;
use ethereum_rust_core::types::LogsFilter;
use ethereum_rust_storage::Store;
use rand::prelude::*;
use serde_json::{json, Value};

use super::logs::LogsRequest;

#[derive(Debug, Clone)]
pub struct FilterRequest {
    pub request_data: LogsRequest,
}

/// Maps IDs to active log filters and their timestamps.
pub type ActiveFilters = Arc<Mutex<HashMap<u64, (u64, LogsFilter)>>>;
impl FilterRequest {
    pub fn handle(
        &self,
        storage: ethereum_rust_storage::Store,
        filters: ActiveFilters,
    ) -> Result<serde_json::Value, crate::utils::RpcErr> {
        let filter = self.request_data.request_to_filter(&storage)?;
        let id: u64 = random();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|unix_time| unix_time.as_secs())
            .map_err(|_err| RpcErr::Internal)?;
        let mut active_filters_guard = filters.lock().unwrap_or_else(|mut poisoned_guard| {
            error!("Logs filtering mutex is poisoned! Cleaning up..");
            **poisoned_guard.get_mut() = HashMap::new();
            filters.clear_poison();
            poisoned_guard.into_inner()
        });

        active_filters_guard.insert(id, (timestamp, filter));
        let as_hex = json!(format!("0x{:x}", id));
        Ok(as_hex)
    }

    pub fn parse(params: &Option<Vec<serde_json::Value>>) -> Result<Self, crate::utils::RpcErr> {
        let filter = LogsRequest::parse(params)?;
        Ok(FilterRequest {
            request_data: filter,
        })
    }

    pub fn stateful_call(
        req: &RpcRequest,
        storage: Store,
        state: ActiveFilters,
    ) -> Result<Value, RpcErr> {
        let request = Self::parse(&req.params)?;
        request.handle(storage, state)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        map_http_requests,
        utils::{
            test_utils::{example_p2p_node, TestDB},
            RpcRequest,
        },
    };
    use ethereum_rust_storage::EngineType;
    use serde_json::json;

    #[test]
    fn filter_request_smoke_test_valid_params() {
        let raw_json = json!(
        {
            "jsonrpc":"2.0",
            "method":"eth_newFilter",
            "params":
            [
                {
                    "fromBlock": "0x1",
                    "toBlock": "0x2",
                    "address": null,
                    "topics": ["0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"]
                }
            ]
                ,"id":1
        });
        run_filter_request_test(raw_json.clone(), EngineType::InMemory);
        run_filter_request_test(raw_json.clone(), EngineType::Libmdbx);
    }

    #[test]
    fn filter_request_smoke_test_valid_null_topics_null_addr() {
        let raw_json = json!(
        {
            "jsonrpc":"2.0",
            "method":"eth_newFilter",
            "params":
            [
                {
                    "fromBlock": "0x1",
                    "toBlock": "0xFF",
                    "topics": null,
                    "address": null
                }
            ]
                ,"id":1
        });
        run_filter_request_test(raw_json.clone(), EngineType::InMemory);
        run_filter_request_test(raw_json.clone(), EngineType::Libmdbx);
    }

    #[test]
    fn filter_request_smoke_test_valid_addr_topic_null() {
        let raw_json = json!(
        {
            "jsonrpc":"2.0",
            "method":"eth_newFilter",
            "params":
            [
                {
                    "fromBlock": "0x1",
                    "toBlock": "0xFF",
                    "topics": null,
                    "address": [ "0xb794f5ea0ba39494ce839613fffba74279579268" ]
                }
            ]
                ,"id":1
        });
        run_filter_request_test(raw_json.clone(), EngineType::InMemory);
        run_filter_request_test(raw_json.clone(), EngineType::Libmdbx);
    }

    #[test]
    #[should_panic]
    fn filter_request_smoke_test_invalid_block_range() {
        let raw_json = json!(
        {
            "jsonrpc":"2.0",
            "method":"eth_newFilter",
            "params":
            [
                {
                    "fromBlock": "0xFFF",
                    "toBlock": "0xA",
                    "topics": null,
                    "address": null
                }
            ]
                ,"id":1
        });
        run_filter_request_test(raw_json.clone(), EngineType::Libmdbx);
    }

    #[test]
    #[should_panic]
    fn filter_request_smoke_test_from_block_missing() {
        let raw_json = json!(
        {
            "jsonrpc":"2.0",
            "method":"eth_newFilter",
            "params":
            [
                {
                    "fromBlock": null,
                    "toBlock": "0xA",
                    "topics": null,
                    "address": null
                }
            ]
                ,"id":1
        });
        run_filter_request_test(raw_json.clone(), EngineType::Libmdbx);
    }

    fn run_filter_request_test(json_req: serde_json::Value, storage_type: EngineType) {
        let node = example_p2p_node();
        let request: RpcRequest = serde_json::from_value(json_req).expect("Test json is incorrect");
        let test_store = TestDB::new(storage_type);
        let response =
            map_http_requests(&request, test_store.build_store(), node, Default::default())
                .unwrap()
                .to_string();
        assert!(response.trim().trim_matches('"').starts_with("0x"))
    }
}
