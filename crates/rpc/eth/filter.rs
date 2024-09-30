use ethereum_rust_core::types::LogsFilter;

use crate::types::block_identifier::BlockIdentifier;
use crate::RpcHandler;

use super::logs::LogsRequest;

#[derive(Debug, Clone)]
pub struct FilterRequest {
    pub filter: LogsRequest,
}

impl RpcHandler for FilterRequest {
    fn parse(params: &Option<Vec<serde_json::Value>>) -> Result<Self, crate::utils::RpcErr> {
        let filter = LogsRequest::parse(params)?;
        Ok(FilterRequest { filter })
    }
    fn handle(
        &self,
        storage: ethereum_rust_storage::Store,
    ) -> Result<serde_json::Value, crate::utils::RpcErr> {
        let LogsFilter {
            from_block,
            to_block,
            address: address_filters,
            topics,
        }: &LogsFilter = &self.filter.request_to_filter(&storage)?;
        let from_block = BlockIdentifier::Number(*from_block);
        let to_block = BlockIdentifier::Number(*to_block);
        let from = BlockIdentifier::resolve_block_number(&from_block, &storage)
            .unwrap()
            .unwrap();
        let to = BlockIdentifier::resolve_block_number(&to_block, &storage)
            .unwrap()
            .unwrap();
        storage.add_filter(from, to, address_filters.clone().unwrap(), &topics[..])?;
        todo!()
    }
}
