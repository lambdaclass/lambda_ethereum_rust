use ethereum_rust_storage::Store;
use serde_json::{json, Value};

use crate::{
    types::fork_choice::{ForkChoiceState, PayloadAttributesV3},
    RpcErr,
};

#[derive(Debug)]
pub struct ForkChoiceUpdatedV3 {
    pub fork_choice_state: ForkChoiceState,
    #[allow(unused)]
    pub payload_attributes: Option<PayloadAttributesV3>,
}

impl ForkChoiceUpdatedV3 {
    pub fn parse(params: &Option<Vec<Value>>) -> Result<ForkChoiceUpdatedV3, RpcErr> {
        let params = params.as_ref().ok_or(RpcErr::BadParams)?;
        if params.len() != 2 {
            return Err(RpcErr::BadParams);
        }
        Ok(ForkChoiceUpdatedV3 {
            fork_choice_state: serde_json::from_value(params[0].clone())?,
            payload_attributes: serde_json::from_value(params[1].clone())?,
        })
    }
}

pub fn forkchoice_updated_v3(
    request: ForkChoiceUpdatedV3,
    storage: Store,
) -> Result<Value, RpcErr> {
    // Just a minimal implementation to pass rpc-compat Hive tests.
    // TODO (#50): Implement `engine_forkchoiceUpdatedV3`
    let safe = storage.get_block_number(request.fork_choice_state.safe_block_hash);
    let finalized = storage.get_block_number(request.fork_choice_state.finalized_block_hash);

    // Check if we already have the blocks stored.
    let (safe_block_number, finalized_block_number) = match (safe, finalized) {
        (Ok(Some(safe)), Ok(Some(finalized))) => (safe, finalized),
        _ => return Err(RpcErr::Internal),
    };

    storage.update_finalized_block_number(finalized_block_number)?;
    storage.update_safe_block_number(safe_block_number)?;
    serde_json::to_value(json!({
        "payloadId": null,
        "payloadStatus": {
            "latestValidHash": null,
            "status": "SYNCING",
            "validationError": null
        }
    }))
    .map_err(|_| RpcErr::Internal)
}
