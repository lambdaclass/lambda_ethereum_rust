use ethereum_rust_blockchain::{
    error::ChainError,
    payload::{create_payload, BuildPayloadArgs},
};
use ethereum_rust_core::{types::BlockHeader, H256, U256};
use ethereum_rust_storage::{error::StoreError, Store};
use serde_json::Value;
use tracing::warn;

use crate::{
    types::{
        fork_choice::{ForkChoiceResponse, ForkChoiceState, PayloadAttributesV3},
        payload::PayloadStatus,
    },
    utils::RpcRequest,
    RpcErr, RpcHandler,
};

#[derive(Debug)]
pub struct ForkChoiceUpdatedV3 {
    pub fork_choice_state: ForkChoiceState,
    #[allow(unused)]
    pub payload_attributes: Option<PayloadAttributesV3>,
}

impl From<ForkChoiceUpdatedV3> for RpcRequest {
    fn from(val: ForkChoiceUpdatedV3) -> Self {
        RpcRequest {
            method: "engine_forkchoiceUpdatedV3".to_string(),
            params: Some(vec![
                serde_json::json!(val.fork_choice_state),
                serde_json::json!(val.payload_attributes),
            ]),
            ..Default::default()
        }
    }
}

impl RpcHandler for ForkChoiceUpdatedV3 {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let params = params
            .as_ref()
            .ok_or(RpcErr::BadParams("No params provided".to_owned()))?;
        if params.len() != 2 {
            return Err(RpcErr::BadParams("Expected 2 params".to_owned()));
        }
        Ok(ForkChoiceUpdatedV3 {
            fork_choice_state: serde_json::from_value(params[0].clone())?,
            payload_attributes: serde_json::from_value(params[1].clone())
                .map_err(|e| RpcErr::InvalidPayloadAttributes(e.to_string()))?,
        })
    }

    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        let error_response = |err_msg: &str| {
            serde_json::to_value(ForkChoiceResponse::from(PayloadStatus::invalid_with_err(
                err_msg,
            )))
            .map_err(|error| RpcErr::Internal(error.to_string()))
        };

        if self.fork_choice_state.head_block_hash.is_zero() {
            return error_response("forkchoice requested update to zero hash");
        }
        // Check if we have the block stored
        let Some(head_block) =
            storage.get_block_header_by_hash(self.fork_choice_state.head_block_hash)?
        else {
            // TODO: We don't yet support syncing
            warn!("[Engine - ForkChoiceUpdatedV3] Fork choice head block not found in store (hash {}).", self.fork_choice_state.head_block_hash);
            return Err(RpcErr::Internal("We don't yet support syncing".to_owned()));
        };
        // Check that we are not being pushed pre-merge
        if let Some(error) = total_difficulty_check(
            &self.fork_choice_state.head_block_hash,
            &head_block,
            &storage,
        )? {
            return error_response(error);
        }
        let canonical_block = storage.get_canonical_block_hash(head_block.number)?;
        let current_block_hash = {
            let current_block_number = storage.get_latest_block_number()?.ok_or(
                RpcErr::Internal("Could not get latest block number".to_owned()),
            )?;
            storage.get_canonical_block_hash(current_block_number)?
        };
        if canonical_block.is_some_and(|h| h != self.fork_choice_state.head_block_hash) {
            // We are still under the assumption that the blocks are only added if they are connected
            // to the canonical chain. That means that for the state to be consistent we only need to
            // check that the safe and finalized ones are in the canonical chain and that the head's parent is too.
            if storage
                .get_canonical_block_hash(head_block.number.saturating_sub(1))?
                .is_some_and(|h| h == head_block.parent_hash)
            {
                storage.set_canonical_block(
                    head_block.number,
                    self.fork_choice_state.head_block_hash,
                )?;
            }
        } else if current_block_hash.is_some_and(|h| h != self.fork_choice_state.head_block_hash) {
            // If the head block is already in our canonical chain, the beacon client is
            // probably resyncing. Ignore the update.
            return serde_json::to_value(PayloadStatus::valid())
                .map_err(|error| RpcErr::Internal(error.to_string()));
        }

        // Set finalized & safe blocks
        set_finalized_block(&self.fork_choice_state.finalized_block_hash, &storage)?;
        set_safe_block(&self.fork_choice_state.safe_block_hash, &storage)?;

        let mut response = ForkChoiceResponse::from(PayloadStatus::valid_with_hash(
            self.fork_choice_state.head_block_hash,
        ));

        // Build block from received payload
        if let Some(attributes) = &self.payload_attributes {
            let chain_config = storage.get_chain_config()?;
            if !chain_config.is_cancun_activated(attributes.timestamp) {
                return Err(RpcErr::UnsuportedFork(
                    "forkChoiceV3 used to build pre-Cancun payload".to_string(),
                ));
            }
            if attributes.timestamp <= head_block.timestamp {
                return Err(RpcErr::InvalidPayloadAttributes(
                    "invalid timestamp".to_string(),
                ));
            }
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
            let payload = match create_payload(&args, &storage) {
                Ok(payload) => payload,
                Err(ChainError::EvmError(error)) => return Err(error.into()),
                // Parent block is guaranteed to be present at this point,
                // so the only errors that may be returned are internal storage errors
                Err(error) => return Err(RpcErr::Internal(error.to_string())),
            };
            storage.add_payload(payload_id, payload)?;
        }

        serde_json::to_value(response).map_err(|error| RpcErr::Internal(error.to_string()))
    }
}

fn total_difficulty_check<'a>(
    head_block_hash: &'a H256,
    head_block: &'a BlockHeader,
    storage: &'a Store,
) -> Result<Option<&'a str>, StoreError> {
    if !head_block.difficulty.is_zero() || head_block.number == 0 {
        let total_difficulty = storage.get_block_total_difficulty(*head_block_hash)?;
        let parent_total_difficulty = storage.get_block_total_difficulty(head_block.parent_hash)?;
        let terminal_total_difficulty = storage.get_chain_config()?.terminal_total_difficulty;
        if terminal_total_difficulty.is_none()
            || total_difficulty.is_none()
            || head_block.number > 0 && parent_total_difficulty.is_none()
        {
            return Ok(Some(
                "total difficulties unavailable for terminal total difficulty check",
            ));
        }
        if total_difficulty.unwrap() < terminal_total_difficulty.unwrap().into() {
            return Ok(Some("refusing beacon update to pre-merge"));
        }
        if head_block.number > 0 && parent_total_difficulty.unwrap() >= U256::zero() {
            return Ok(Some(
                "parent block is already post terminal total difficulty",
            ));
        }
    }
    Ok(None)
}

fn set_finalized_block(finalized_block_hash: &H256, storage: &Store) -> Result<(), RpcErr> {
    if !finalized_block_hash.is_zero() {
        // If the finalized block is not in our canonical tree, something is wrong
        let Some(finalized_block) = storage.get_block_by_hash(*finalized_block_hash)? else {
            return Err(RpcErr::InvalidForkChoiceState(
                "final block not available in database".to_string(),
            ));
        };

        if !storage
            .get_canonical_block_hash(finalized_block.header.number)?
            .is_some_and(|ref h| h == finalized_block_hash)
        {
            return Err(RpcErr::InvalidForkChoiceState(
                "final block not in canonical chain".to_string(),
            ));
        }
        // Set the finalized block
        storage.update_finalized_block_number(finalized_block.header.number)?;
    }
    Ok(())
}

fn set_safe_block(safe_block_hash: &H256, storage: &Store) -> Result<(), RpcErr> {
    if !safe_block_hash.is_zero() {
        // If the safe block is not in our canonical tree, something is wrong
        let Some(safe_block) = storage.get_block_by_hash(*safe_block_hash)? else {
            return Err(RpcErr::InvalidForkChoiceState(
                "safe block not available in database".to_string(),
            ));
        };

        if !storage
            .get_canonical_block_hash(safe_block.header.number)?
            .is_some_and(|ref h| h == safe_block_hash)
        {
            return Err(RpcErr::InvalidForkChoiceState(
                "safe block not in canonical chain".to_string(),
            ));
        }
        // Set the safe block
        storage.update_safe_block_number(safe_block.header.number)?;
    }
    Ok(())
}
