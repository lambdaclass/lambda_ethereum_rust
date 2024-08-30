pub mod fork_choice;
pub mod payload;

use crate::RpcErr;
use serde_json::{json, Value};

pub type ExchangeCapabilitiesRequest = Vec<String>;

pub fn exchange_capabilities(capabilities: &ExchangeCapabilitiesRequest) -> Result<Value, RpcErr> {
    Ok(json!(capabilities))
}
