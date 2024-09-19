pub mod exchange_transition_config;
pub mod fork_choice;
pub mod payload;

use crate::{RpcErr, RpcHandler, Store};
use ethereum_rust_storage::StoreEngine;
use serde_json::{json, Value};

pub type ExchangeCapabilitiesRequest = Vec<String>;

impl RpcHandler for ExchangeCapabilitiesRequest {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        params
            .as_ref()
            .ok_or(RpcErr::BadParams)?
            .first()
            .ok_or(RpcErr::BadParams)
            .and_then(|v| serde_json::from_value(v.clone()).map_err(|_| RpcErr::BadParams))
    }

    fn handle<E: StoreEngine>(&self, _storage: Store<E>) -> Result<Value, RpcErr> {
        Ok(json!(*self))
    }
}
