use ethereum_rust_blockchain::{
    error::ChainError,
    new_head,
    payload::{build_payload, BuildPayloadArgs},
};
use ethereum_rust_core::types::{BlockHash, BlockHeader, BlockNumber};
use ethereum_rust_core::{H256, U256};
use ethereum_rust_storage::{error::StoreError, Store};
use serde_json::{json, Value};
use tracing::warn;

use crate::{
    types::{
        fork_choice::{ForkChoiceResponse, ForkChoiceState, PayloadAttributesV3},
        payload::PayloadStatus,
    },
    RpcErr, RpcHandler,
};

#[derive(Debug)]
pub struct ForkChoiceUpdatedV3 {
    pub fork_choice_state: ForkChoiceState,
    #[allow(unused)]
    pub payload_attributes: Option<PayloadAttributesV3>,
}

impl RpcHandler for ForkChoiceUpdatedV3 {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let params = params.as_ref().ok_or(RpcErr::BadParams)?;
        if params.len() != 2 {
            return Err(RpcErr::BadParams);
        }
        Ok(ForkChoiceUpdatedV3 {
            fork_choice_state: serde_json::from_value(params[0].clone())?,
            payload_attributes: serde_json::from_value(params[1].clone())?,
        })
    }

    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        let error_response = |err_msg: &str| {
            serde_json::to_value(ForkChoiceResponse::from(PayloadStatus::invalid_with_err(
                err_msg,
            )))
            .map_err(|_| RpcErr::Internal)
        };

        // Build block from received payload
        let mut response = ForkChoiceResponse::from(PayloadStatus::valid_with_hash(
            self.fork_choice_state.head_block_hash,
        ));

        if let Some(attributes) = &self.payload_attributes {
            let args = BuildPayloadArgs {
                parent: self.fork_choice_state.head_block_hash,
                timestamp: attributes.timestamp,
                fee_recipient: attributes.suggested_fee_recipient,
                random: attributes.prev_randao,
                withdrawals: attributes.withdrawals.clone(),
                beacon_root: Some(attributes.parent_beacon_block_root),
                version: 3,
            };
            let payload_id = args.id();
            response.set_id(payload_id);
            let payload = match build_payload(&args, &storage) {
                Ok(payload) => payload,
                Err(ChainError::EvmError(error)) => return Err(error.into()),
                // Parent block is guaranteed to be present at this point,
                // so the only errors that may be returned are internal storage errors
                _ => return Err(RpcErr::Internal),
            };
            storage.add_payload(payload_id, payload)?;
        }

        // TODO: Map error better.
        new_head(
            &storage,
            self.fork_choice_state.head_block_hash,
            self.fork_choice_state.safe_block_hash,
            self.fork_choice_state.finalized_block_hash,
        )
        .map_err(|_| RpcErr::Internal)?;

        serde_json::to_value(response).map_err(|_| RpcErr::Internal)
    }
}

fn invalid_fork_choice_state() -> Result<Value, RpcErr> {
    serde_json::to_value(json!({"error": {"code": -38002, "message": "Invalid forkchoice state"}}))
        .map_err(|_| RpcErr::Internal)
}
