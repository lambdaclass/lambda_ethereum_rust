use ethereum_rust_blockchain::is_canonical;
use ethereum_rust_core::types::{BlockHash, BlockHeader, BlockNumber};
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
                return syncing_response();
            }
        };

        // Check that the headers are in the correct order.
        if finalized.number > safe.number || safe.number > head.number {
            return invalid_fork_choice_state();
        }

        // We look if each of the key blocks is canonical in our chain. If two of them are, we already know they are connected and can
        // skip the check.
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
        let head_canonical = is_canonical(
            &storage,
            head.number,
            self.fork_choice_state.head_block_hash,
        )?;

        // Ancestry checks if for the necessary. Empty ancestries will mean that they don't need to be updated.
        let head_ancestry = if head_canonical && safe_canonical {
            Some(Vec::new())
        } else {
            find_ancestry(&storage, &safe, &head).map_err(|_| RpcErr::Internal)?
        };

        let safe_ancestry = if safe_canonical && finalized_canonical {
            Some(Vec::new())
        } else {
            find_ancestry(&storage, &finalized, &safe).map_err(|_| RpcErr::Internal)?
        };

        match (head_ancestry, safe_ancestry) {
            (Some(ha), Some(sa)) => {
                for (number, hash) in sa {
                    storage.set_canonical_block(number, hash)?;
                }

                for (number, hash) in ha {
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
                syncing_response()
            }
            _ => invalid_fork_choice_state(),
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

// Find branch of the blockchain connecting two blocks. If the blocks are connected through
// parent hashes, then a vector of number-hash pairs is returned for the branch. If they are not
// connected, an error is returned.
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
