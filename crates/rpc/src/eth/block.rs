use serde::Deserialize;
use serde_json::Value;

use crate::utils::RpcErr;

pub struct GetBlockByNumberRequest {
    pub block: BlockIdentifier,
    pub hydrated: bool,
}

#[derive(Deserialize)]
enum BlockIdentifier {
    Number(u64),
    Tag(BlockTag),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
enum BlockTag {
    Earliest,
    Finalized,
    Safe,
    Latest,
    Pending,
}

impl GetBlockByNumberRequest {
    pub fn parse(params: &Option<Vec<Value>>) -> Option<GetBlockByNumberRequest> {
        let params = params.as_ref()?;
        if params.len() != 2 {
            return None;
        };
        Some(GetBlockByNumberRequest {
            block: serde_json::from_value(params[0].clone()).ok()?,
            hydrated: serde_json::from_value(params[1].clone()).ok()?,
        })
    }
}

pub fn get_block_by_number(request: &GetBlockByNumberRequest) -> Result<Value, RpcErr> {
    Ok(Value::Null)
}
