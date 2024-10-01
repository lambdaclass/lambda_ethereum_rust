use ethereum_rust_blockchain::is_canonical;
use ethereum_rust_blockchain::payload::{build_payload, BuildPayloadArgs};
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

        if self.fork_choice_state.head_block_hash.is_zero() {
            return error_response("forkchoice requested update to zero hash");
        }
        let finalized_header_res =
            storage.get_block_header_by_hash(self.fork_choice_state.finalized_block_hash)?;
        let safe_header_res =
            storage.get_block_header_by_hash(self.fork_choice_state.safe_block_hash)?;
        let head_header_res =
            storage.get_block_header_by_hash(self.fork_choice_state.head_block_hash)?;

        // Check that we already have all the needed blocks stored and that we have the ancestors
        // if we have the descendants, as we are working on the assumption that we only add block
        // if they are connected to the canonical chain.
        let (finalized, safe, head) = match (finalized_header_res, safe_header_res, head_header_res)
        {
            (None, Some(_), _) => return invalid_fork_choice_state(),
            (_, None, Some(_)) => return invalid_fork_choice_state(),
            (Some(f), Some(s), Some(h)) => (f, s, h),
            _ => {
                warn!("[Engine - ForkChoiceUpdatedV3] Fork choice block not found in store (hash {}).", self.fork_choice_state.head_block_hash);
                return serde_json::to_value(PayloadStatus::syncing())
                    .map_err(|_| RpcErr::Internal);
            }
        };
        // Check that we are not being pushed pre-merge
        if let Some(error) =
            total_difficulty_check(&self.fork_choice_state.head_block_hash, &head, &storage)?
        {
            return error_response(error);
        }

        // Check that the headers are in the correct order.
        if finalized.number > safe.number || safe.number > head.number {
            return invalid_fork_choice_state();
        }

        // If the head block is already in our canonical chain, the beacon client is
        // probably resyncing. Ignore the update.
        if is_canonical(
            &storage,
            head.number,
            self.fork_choice_state.head_block_hash,
        )? {
            return serde_json::to_value(PayloadStatus::valid()).map_err(|_| RpcErr::Internal);
        }

        // If both finalized and safe blocks are canonical, we can skip the ancestry check.
        let finalized_canonical = is_canonical(
            &storage,
            finalized.number,
            self.fork_choice_state.finalized_block_hash,
        )?;
        let safe_canonical = is_canonical(
            &storage,
            safe.number,
            self.fork_choice_state.safe_block_hash,
        )?;

        // Find out if blocks are correctly connected.
        let Some(head_ancestry) =
            find_ancestry(&storage, &safe, &head).map_err(|_| RpcErr::Internal)?
        else {
            return Err(RpcErr::InvalidForkChoiceState(
                "Head and Safe blocks are not related".to_string(),
            ));
        };

        let safe_ancestry = if safe_canonical && finalized_canonical {
            // Skip check. We will not canonize anything between safe and finalized blocks.
            Vec::new()
        } else {
            let Some(ancestry) =
                find_ancestry(&storage, &finalized, &safe).map_err(|_| RpcErr::Internal)?
            else {
                return Err(RpcErr::InvalidForkChoiceState(
                    "Head and Safe blocks are not related".to_string(),
                ));
            };
            ancestry
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
            let payload = build_payload(&args, &storage)?;
            storage.add_payload(payload_id, payload)?;
        }

        // Canonize blocks from both ancestries.
        for (number, hash) in safe_ancestry {
            storage.set_canonical_block(number, hash)?;
        }

        for (number, hash) in head_ancestry {
            storage.set_canonical_block(number, hash)?;
        }

        storage.set_canonical_block(head.number, self.fork_choice_state.head_block_hash)?;
        storage.set_canonical_block(safe.number, self.fork_choice_state.safe_block_hash)?;
        storage.set_canonical_block(
            finalized.number,
            self.fork_choice_state.finalized_block_hash,
        )?;

        storage.update_finalized_block_number(finalized.number)?;
        storage.update_safe_block_number(safe.number)?;
        serde_json::to_value(response).map_err(|_| RpcErr::Internal)
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

fn invalid_fork_choice_state() -> Result<Value, RpcErr> {
    serde_json::to_value(json!({"error": {"code": -38002, "message": "Invalid forkchoice state"}}))
        .map_err(|_| RpcErr::Internal)
}

// Find branch of the blockchain connecting two blocks. If the blocks are connected through
// parent hashes, then a vector of number-hash pairs is returned for the branch. If they are not
// connected, an error is returned.
//
// Return values:
// - Err(StoreError): a db-related error happened.
// - Ok(None): the headers are not related by ancestry.
// - Ok(Some([])): the headers are the same block.
// - Ok(Some(branch)): the "branch" is a sequence of blocks that connects the ancestor and the
//   descendant.
fn find_ancestry(
    storage: &Store,
    ancestor: &BlockHeader,
    descendant: &BlockHeader,
) -> Result<Option<Vec<(BlockNumber, BlockHash)>>, StoreError> {
    let mut block_number = descendant.number;
    let mut found = false;
    let descendant_hash = descendant.compute_block_hash();
    let ancestor_hash = ancestor.compute_block_hash();
    let mut header = descendant.clone();
    let mut branch = Vec::new();

    if ancestor.number == descendant.number {
        if ancestor_hash == descendant_hash {
            return Ok(Some(branch));
        } else {
            return Ok(None);
        }
    }

    while block_number > ancestor.number && !found {
        block_number -= 1;
        let parent_hash = header.parent_hash;

        // Check that the parent exists.
        let parent_header = match storage.get_block_header_by_hash(parent_hash) {
            Ok(Some(header)) => header,
            Ok(None) => return Ok(None),
            Err(error) => return Err(error),
        };

        if block_number == ancestor.number {
            if ancestor_hash == descendant_hash {
                found = true;
            } else {
                return Ok(None);
            }
        } else {
            branch.push((block_number, parent_hash));
        }

        header = parent_header;
    }

    if found {
        Ok(Some(branch))
    } else {
        Ok(None)
    }
}
