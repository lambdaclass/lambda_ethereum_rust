use ethereum_rust_core::{types::BuildPayloadArgs, U256};
use ethereum_rust_storage::Store;
use serde_json::Value;

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
        if self.fork_choice_state.head_block_hash.is_zero() {
            return Ok(error_response("forkchoice requested update to zero hash")?);
        }
        // Check if we have the block stored
        let Some(head_block) = storage.get_block_by_hash(self.fork_choice_state.head_block_hash)?
        else {
            // TODO: We don't yet support syncing
            return Err(RpcErr::Internal);
        };
        // Check that we are not being pushed pre-merge
        if !head_block.header.difficulty.is_zero() || head_block.header.number == 0 {
            let total_difficulty =
                storage.get_block_total_difficulty(self.fork_choice_state.head_block_hash)?;
            let parent_total_difficulty =
                storage.get_block_total_difficulty(head_block.header.parent_hash)?;
            let terminal_total_difficulty = storage.get_chain_config()?.terminal_total_difficulty;
            if terminal_total_difficulty.is_none()
                || total_difficulty.is_none()
                || head_block.header.number > 0 && parent_total_difficulty.is_none()
            {
                return Ok(error_response(
                    "total difficulties unavailable for terminal total difficulty check",
                )?);
            }
            if total_difficulty.unwrap() < terminal_total_difficulty.unwrap().into() {
                return Ok(error_response("refusing beacon update to pre-merge")?);
            }
            if head_block.header.number > 0 && parent_total_difficulty.unwrap() >= U256::zero() {
                return Ok(error_response(
                    "parent block is already post terminal total difficulty",
                )?);
            }
        }
        let canonical_block = storage.get_canonical_block(head_block.header.number)?;
        let current_block_hash = {
            let current_block_number =
                storage.get_latest_block_number()?.ok_or(RpcErr::Internal)?;
            storage.get_canonical_block(current_block_number)?
        };
        if canonical_block.is_some_and(|h| h != self.fork_choice_state.head_block_hash) {
            // TODO: We don't handle re-orgs yet
            return Err(RpcErr::Internal);
        } else if current_block_hash.is_some_and(|h| h != self.fork_choice_state.head_block_hash) {
            // If the head block is already in our canonical chain, the beacon client is
            // probably resyncing. Ignore the update.
            return Ok(serde_json::to_value(PayloadStatus::valid()).map_err(|_| RpcErr::Internal)?);
        }

        // Process finalized block
        if !self.fork_choice_state.finalized_block_hash.is_zero() {
            // If the finalized block is not in our canonical tree, something is wrong
            let Some(finalized_block) =
                storage.get_block_by_hash(self.fork_choice_state.finalized_block_hash)?
            else {
                return Ok(error_response("final block not available in database")?);
            };

            if !storage
                .get_canonical_block(finalized_block.header.number)?
                .is_some_and(|h| h == self.fork_choice_state.finalized_block_hash)
            {
                return Ok(error_response("final block not in canonical chain")?);
            }
            // Set the finalized block
            storage.update_finalized_block_number(finalized_block.header.number)?;
        }

        // Process safe block
        if !self.fork_choice_state.safe_block_hash.is_zero() {
            // If the safe block is not in our canonical tree, something is wrong
            let Some(safe_block) =
                storage.get_block_by_hash(self.fork_choice_state.safe_block_hash)?
            else {
                return Ok(error_response("safe block not available in database")?);
            };

            if !storage
                .get_canonical_block(safe_block.header.number)?
                .is_some_and(|h| h == self.fork_choice_state.safe_block_hash)
            {
                return Ok(error_response("safe block not in canonical chain")?);
            }
            // Set the safe block
            storage.update_safe_block_number(safe_block.header.number)?;
        }
        let mut response = ForkChoiceResponse::from(PayloadStatus::valid_with_hash(
            self.fork_choice_state.head_block_hash,
        ));

        // TODO: Build block
        if let Some(attributes) = &self.payload_attributes {
            let payload = BuildPayloadArgs {
                parent: self.fork_choice_state.head_block_hash,
                timestamp: attributes.timestamp,
                fee_recipient: attributes.suggested_fee_recipient,
                random: attributes.prev_randao,
                withdrawals: attributes.withdrawals.clone(),
                beacon_root: Some(attributes.parent_beacon_block_root),
                version: 3,
            };
            let payload_id = payload.id();
            response.set_id(payload_id);
        }

        Ok(serde_json::to_value(response).map_err(|_| RpcErr::Internal)?)
    }
}
