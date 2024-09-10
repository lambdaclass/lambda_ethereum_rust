use serde_json::Value;
use tracing::info;

use crate::{eth::block::BlockIdentifier, utils::RpcErr, RpcHandler};
use ethereum_rust_core::{rlp::encode::RLPEncode, types::Block};
use ethereum_rust_storage::Store;

pub struct GetRawBlock {
    pub block: BlockIdentifier,
}

impl RpcHandler for GetRawBlock {
    fn parse(params: &Option<Vec<Value>>) -> Option<GetRawBlock> {
        let params = params.as_ref()?;
        if params.len() != 1 {
            return None;
        }
        Some(GetRawBlock {
            block: serde_json::from_value(params[0].clone()).ok()?,
        })
    }

    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        info!("Requested raw block: {}", self.block);
        let block_number = match self.block.resolve_block_number(&storage)? {
            Some(block_number) => block_number,
            _ => return Ok(Value::Null),
        };
        let header = storage.get_block_header(block_number)?;
        let body = storage.get_block_body(block_number)?;
        let (header, body) = match (header, body) {
            (Some(header), Some(body)) => (header, body),
            _ => return Ok(Value::Null),
        };
        let block = Block {
            header: header.clone(),
            body: body.clone(),
        }
        .encode_to_vec();

        serde_json::to_value(format!("0x{}", &hex::encode(block))).map_err(|_| RpcErr::Internal)
    }
}
