use ethereum_rust_core::types::{BlockHash, BlockNumber};
use ethereum_rust_storage::{error::StoreError, Store};
use serde_json::{json, Value};
use tracing::warn;

use crate::{
    types::fork_choice::{ForkChoiceState, PayloadAttributesV3},
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
        // Just a minimal implementation to pass rpc-compat Hive tests.
        // TODO (#50): Implement `engine_forkchoiceUpdatedV3`
        let safe = storage.get_block_number(self.fork_choice_state.safe_block_hash);
        let finalized = storage.get_block_number(self.fork_choice_state.finalized_block_hash);

        // Check if we already have the blocks stored.
        let (safe_block_number, finalized_block_number) = match (safe, finalized) {
            (Ok(Some(safe)), Ok(Some(finalized))) => (safe, finalized),
            _ => return Err(RpcErr::Internal),
        };

        // Check if the payload is available for the new head hash.
        let header = match storage.get_block_header_by_hash(self.fork_choice_state.head_block_hash)
        {
            Ok(Some(header)) => header,
            Ok(None) => {
                warn!("[Engine - ForkChoiceUpdatedV3] Fork choice head block not found in store (hash {}).", self.fork_choice_state.head_block_hash);
                return syncing_response();
            }
            _ => return Err(RpcErr::Internal),
        };

        // We are still under the assumption that the blocks are only added if they are connected
        // to the canonical chain. That means that for the state to be consistent we only need to
        // check that the safe and finalized ones are in the canonical chain and that the heads parent is too.

        let head_valid = is_canonical(&storage, header.number - 1, header.parent_hash)
            .map_err(|_| RpcErr::Internal)?;

        let safe_valid = is_canonical(
            &storage,
            safe_block_number,
            self.fork_choice_state.safe_block_hash,
        )
        .map_err(|_| RpcErr::Internal)?;

        let finalized_valid = is_canonical(
            &storage,
            finalized_block_number,
            self.fork_choice_state.finalized_block_hash,
        )
        .map_err(|_| RpcErr::Internal)?;

        if head_valid && safe_valid && finalized_valid {
            storage.set_canonical_block(header.number, self.fork_choice_state.head_block_hash)?;
            storage.update_finalized_block_number(finalized_block_number)?;
            storage.update_safe_block_number(safe_block_number)?;
            syncing_response()
        } else {
            invalid_fork_choice_state()
        }
    }
}

fn syncing_response() -> Result<Value, RpcErr> {
    serde_json::to_value(json!({
    "payloadId": null,
    "payloadStatus": {
        "latestValidHash": null,
        "status": "SYNCING",
        "validationError": null
    }}))
    .map_err(|_| RpcErr::Internal)
}

fn invalid_fork_choice_state() -> Result<Value, RpcErr> {
    serde_json::to_value(json!({"error": {"code": -38002, "message": "Invalid forkchoice state"}}))
        .map_err(|_| RpcErr::Internal)
}

fn is_canonical(
    store: &Store,
    block_number: BlockNumber,
    block_hash: BlockHash,
) -> Result<bool, StoreError> {
    match store.get_canonical_block_hash(block_number)? {
        Some(hash) if hash == block_hash => Ok(true),
        _ => Ok(false),
    }
}
