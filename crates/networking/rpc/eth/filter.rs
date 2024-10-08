use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use ethereum_rust_storage::Store;
use tracing::error;

use crate::utils::{RpcErr, RpcRequest};
use crate::RpcHandler;
use rand::prelude::*;
use serde_json::{json, Value};

use super::logs::LogsFilter;

#[derive(Debug, Clone)]
pub struct FilterRequest {
    pub request_data: LogsFilter,
}

/// Maps IDs to active log filters and their timestamps.
pub type ActiveFilters = Arc<Mutex<HashMap<u64, (u64, LogsFilter)>>>;
impl FilterRequest {
    pub fn handle(
        &self,
        storage: ethereum_rust_storage::Store,
        filters: ActiveFilters,
    ) -> Result<serde_json::Value, crate::utils::RpcErr> {
        let from = self
            .request_data
            .from_block
            .resolve_block_number(&storage)?
            .ok_or(RpcErr::WrongParam("fromBlock".to_string()))?;
        let to = self
            .request_data
            .to_block
            .resolve_block_number(&storage)?
            .ok_or(RpcErr::WrongParam("toBlock".to_string()))?;

        if (from..=to).is_empty() {
            return Err(RpcErr::BadParams("Invalid block range".to_string()));
        }

        let id: u64 = random();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|unix_time| unix_time.as_secs())
            .map_err(|error| RpcErr::Internal(error.to_string()))?;
        let mut active_filters_guard = filters.lock().unwrap_or_else(|mut poisoned_guard| {
            error!("THREAD CRASHED WITH MUTEX TAKEN; SYSTEM MIGHT BE UNSTABLE");
            **poisoned_guard.get_mut() = HashMap::new();
            filters.clear_poison();
            poisoned_guard.into_inner()
        });

        active_filters_guard.insert(id, (timestamp, self.request_data.clone()));
        let as_hex = json!(format!("0x{:x}", id));
        Ok(as_hex)
    }

    pub fn parse(params: &Option<Vec<serde_json::Value>>) -> Result<Self, RpcErr> {
        let filter = LogsFilter::parse(params)?;
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
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use crate::{
        eth::logs::{AddressFilter, TopicFilter},
        map_http_requests,
    };
    use crate::{
        types::block_identifier::BlockIdentifier,
        utils::{test_utils::example_p2p_node, RpcRequest},
    };
    use ethereum_rust_storage::{EngineType, Store};
    use serde_json::json;

    use super::ActiveFilters;

    #[test]
    fn filter_request_smoke_test_valid_params() {
        let filter_req_params = json!(
                {
                    "fromBlock": "0x1",
                    "toBlock": "0x2",
                    "address": null,
                    "topics": ["0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"]
                }
        );
        let raw_json = json!(
        {
            "jsonrpc":"2.0",
            "method":"eth_newFilter",
            "params":
            [
                filter_req_params.clone()
            ]
                ,"id":1
        });
        let filters = Arc::new(Mutex::new(HashMap::new()));
        let id = run_filter_request_test(raw_json.clone(), filters.clone());
        let filters = filters.lock().unwrap();
        assert!(filters.len() == 1);
        let (_, filter) = filters.clone().get(&id).unwrap().clone();
        assert!(matches!(filter.from_block, BlockIdentifier::Number(1)));
        assert!(matches!(filter.to_block, BlockIdentifier::Number(2)));
        assert!(filter.address_filters.is_none());
        assert!(matches!(&filter.topics[..], [TopicFilter::Topic(_)]));
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
        let filters = Arc::new(Mutex::new(HashMap::new()));
        let id = run_filter_request_test(raw_json.clone(), filters.clone());
        let filters = filters.lock().unwrap();
        assert!(filters.len() == 1);
        let (_, filter) = filters.clone().get(&id).unwrap().clone();
        assert!(matches!(filter.from_block, BlockIdentifier::Number(1)));
        assert!(matches!(filter.to_block, BlockIdentifier::Number(255)));
        assert!(filter.address_filters.is_none());
        assert!(matches!(&filter.topics[..], []));
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
        let filters = Arc::new(Mutex::new(HashMap::new()));
        let id = run_filter_request_test(raw_json.clone(), filters.clone());
        let filters = filters.lock().unwrap();
        assert!(filters.len() == 1);
        let (_, filter) = filters.clone().get(&id).unwrap().clone();
        assert!(matches!(filter.from_block, BlockIdentifier::Number(1)));
        assert!(matches!(filter.to_block, BlockIdentifier::Number(255)));
        assert!(matches!(
            filter.address_filters.unwrap(),
            AddressFilter::Many(_)
        ));
        assert!(matches!(&filter.topics[..], []));
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
        run_filter_request_test(raw_json.clone(), Default::default());
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
        run_filter_request_test(raw_json.clone(), Default::default());
    }

    fn run_filter_request_test(json_req: serde_json::Value, filters_pointer: ActiveFilters) -> u64 {
        let node = example_p2p_node();
        let request: RpcRequest = serde_json::from_value(json_req).expect("Test json is incorrect");
        let response = map_http_requests(
            &request,
            Store::new("in-mem", EngineType::InMemory).unwrap(),
            node,
            filters_pointer.clone(),
        )
        .unwrap()
        .to_string();
        // Check id is a hex num.
        let trimmed_id = response.trim().trim_matches('"');
        assert!(trimmed_id.starts_with("0x"));
        let hex = trimmed_id.trim_start_matches("0x");
        let parsed = u64::from_str_radix(hex, 16);
        assert!(u64::from_str_radix(hex, 16).is_ok());
        parsed.unwrap()
    }
}
