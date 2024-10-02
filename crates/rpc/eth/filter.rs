use std::time::{SystemTime, UNIX_EPOCH};

use crate::utils::{parse_json_hex, RpcErr};
use crate::RpcHandler;
use rand::prelude::*;
use serde_json::json;

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
        let as_hex = json!(format!("0x{:x}", id));
        Ok(as_hex)
    }
}
#[derive(Debug, Clone)]
pub struct FilterUninstallRequest {
    pub filter_id: u64,
}

impl RpcHandler for FilterUninstallRequest {
    fn parse(params: &Option<Vec<serde_json::Value>>) -> Result<Self, RpcErr> {
        match params.as_deref() {
            Some([param]) => {
                let param = param.as_object().ok_or(RpcErr::BadParams)?;
                let id = param
                    .get("id")
                    .ok_or(RpcErr::MissingParam("id".to_string()))?;
                let filter_id = parse_json_hex(id).unwrap();
                Ok(FilterUninstallRequest { filter_id })
            }
            _ => Err(RpcErr::BadParams),
        }
    }

    fn handle(&self, storage: ethereum_rust_storage::Store) -> Result<serde_json::Value, RpcErr> {
        todo!()
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
        let response = map_http_requests(&request, test_store.build_store(), node)
            .unwrap()
            .to_string();
        assert!(response.trim().trim_matches('"').starts_with("0x"))
    }
}
