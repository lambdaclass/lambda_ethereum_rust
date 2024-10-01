use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::block_identifier::BlockIdentifier;
use crate::utils::RpcErr;
use crate::RpcHandler;
use ethereum_rust_core::types::LogsFilter;
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
        return Ok(as_hex.into());
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        map_eth_requests, map_http_requests,
        utils::{
            test_utils::{example_p2p_node, in_mem_test_db, test_db},
            RpcRequest,
        },
    };
    use ethereum_rust_net::types::Node;
    use ethereum_rust_storage::Store;
    use serde::Deserialize;
    use serde_json::json;

    #[test]
    fn filter_request_smoke_test_in_mem() {
        run_filter_request_test(in_mem_test_db(), example_p2p_node());
        run_filter_request_test(test_db(), example_p2p_node());
    }

    fn run_filter_request_test(storage: Store, test_node: Node) {
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
        let request: RpcRequest = serde_json::from_value(raw_json).expect("Test json is incorrect");
        let response = map_http_requests(&request, storage, test_node)
            .unwrap()
            .to_string();
        assert!(response.trim().trim_matches('"').starts_with("0x"))
    }
}
