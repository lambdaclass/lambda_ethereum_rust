use ethrex_blockchain::{
    error::{ChainError, InvalidForkChoice},
    fork_choice::apply_fork_choice,
    latest_canonical_block_hash,
    payload::{create_payload, BuildPayloadArgs},
};
use ethrex_core::types::BlockHeader;
use serde_json::Value;
use tracing::{info, warn};

use crate::{
    types::{
        fork_choice::{ForkChoiceResponse, ForkChoiceState, PayloadAttributes},
        payload::PayloadStatus,
    },
    utils::RpcRequest,
    RpcApiContext, RpcErr, RpcHandler,
};

#[derive(Debug)]
pub struct ForkChoiceUpdatedV2 {
    pub fork_choice_state: ForkChoiceState,
    pub payload_attributes: Option<PayloadAttributes>,
}

impl RpcHandler for ForkChoiceUpdatedV2 {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let (fork_choice_state, payload_attributes) = parse(params)?;

        Ok(ForkChoiceUpdatedV2 {
            fork_choice_state,
            payload_attributes,
        })
    }

    fn handle(&self, context: RpcApiContext) -> Result<Value, RpcErr> {
        let (head_block_opt, mut response) =
            handle_forkchoice(&self.fork_choice_state, context.clone(), 2)?;
        if let (Some(head_block), Some(attributes)) = (head_block_opt, &self.payload_attributes) {
            validate_v2(attributes, head_block, &context)?;
            let payload_id = build_payload(attributes, context, &self.fork_choice_state, 2)?;
            response.set_id(payload_id);
        }

        serde_json::to_value(response).map_err(|error| RpcErr::Internal(error.to_string()))
    }
}

#[derive(Debug)]
pub struct ForkChoiceUpdatedV3 {
    pub fork_choice_state: ForkChoiceState,
    pub payload_attributes: Option<PayloadAttributes>,
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
        let (fork_choice_state, payload_attributes) = parse(params)?;

        Ok(ForkChoiceUpdatedV3 {
            fork_choice_state,
            payload_attributes,
        })
    }

    fn handle(&self, context: RpcApiContext) -> Result<Value, RpcErr> {
        let (head_block_opt, mut response) =
            handle_forkchoice(&self.fork_choice_state, context.clone(), 2)?;
        if let (Some(head_block), Some(attributes)) = (head_block_opt, &self.payload_attributes) {
            validate_v3(attributes, head_block, &context)?;
            let payload_id = build_payload(attributes, context, &self.fork_choice_state, 3)?;
            response.set_id(payload_id);
        }

        serde_json::to_value(response).map_err(|error| RpcErr::Internal(error.to_string()))
    }
}

fn parse(
    params: &Option<Vec<Value>>,
) -> Result<(ForkChoiceState, Option<PayloadAttributes>), RpcErr> {
    let params = params
        .as_ref()
        .ok_or(RpcErr::BadParams("No params provided".to_owned()))?;
    if params.len() != 2 {
        return Err(RpcErr::BadParams("Expected 2 params".to_owned()));
    }

    let forkchoice_state: ForkChoiceState = serde_json::from_value(params[0].clone())?;
    // if there is an error when parsing, set to None
    let payload_attributes: Option<PayloadAttributes> =
        if let Ok(attributes) = serde_json::from_value::<PayloadAttributes>(params[1].clone()) {
            Some(attributes)
        } else {
            None
        };

    Ok((forkchoice_state, payload_attributes))
}

fn handle_forkchoice(
    fork_choice_state: &ForkChoiceState,
    context: RpcApiContext,
    version: usize,
) -> Result<(Option<BlockHeader>, ForkChoiceResponse), RpcErr> {
    info!(
        "New fork choice request v{} with head: {}, safe: {}, finalized: {}.",
        version,
        fork_choice_state.head_block_hash,
        fork_choice_state.safe_block_hash,
        fork_choice_state.finalized_block_hash
    );

    match apply_fork_choice(
        &context.storage,
        fork_choice_state.head_block_hash,
        fork_choice_state.safe_block_hash,
        fork_choice_state.finalized_block_hash,
    ) {
        Ok(head) => Ok((
            Some(head),
            ForkChoiceResponse::from(PayloadStatus::valid_with_hash(
                fork_choice_state.head_block_hash,
            )),
        )),
        Err(forkchoice_error) => {
            let forkchoice_response = match forkchoice_error {
                InvalidForkChoice::NewHeadAlreadyCanonical => {
                    ForkChoiceResponse::from(PayloadStatus::valid_with_hash(
                        latest_canonical_block_hash(&context.storage).unwrap(),
                    ))
                }
                InvalidForkChoice::Syncing => {
                    // Start sync
                    let current_number = context.storage.get_latest_block_number()?.unwrap();
                    let Some(current_head) =
                        context.storage.get_canonical_block_hash(current_number)?
                    else {
                        return Err(RpcErr::Internal(
                            "Missing latest canonical block".to_owned(),
                        ));
                    };
                    let sync_head = fork_choice_state.head_block_hash;
                    tokio::spawn(async move {
                        // If we can't get hold of the syncer, then it means that there is an active sync in process
                        if let Ok(mut syncer) = context.syncer.try_lock() {
                            syncer
                                .start_sync(current_head, sync_head, context.storage.clone())
                                .await
                        }
                    });
                    ForkChoiceResponse::from(PayloadStatus::syncing())
                }
                reason => {
                    warn!("Invalid fork choice state. Reason: {:#?}", reason);
                    return Err(RpcErr::InvalidForkChoiceState(reason.to_string()));
                }
            };
            Ok((None, forkchoice_response))
        }
    }
}

fn validate_v3(
    attributes: &PayloadAttributes,
    head_block: BlockHeader,
    context: &RpcApiContext,
) -> Result<(), RpcErr> {
    let chain_config = context.storage.get_chain_config()?;
    if attributes.parent_beacon_block_root.is_none() {
        return Err(RpcErr::InvalidPayloadAttributes(
            "Null Parent Beacon Root".to_string(),
        ));
    }
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
    Ok(())
}

fn validate_v2(
    attributes: &PayloadAttributes,
    head_block: BlockHeader,
    context: &RpcApiContext,
) -> Result<(), RpcErr> {
    let chain_config = context.storage.get_chain_config()?;
    if attributes.parent_beacon_block_root.is_some() {
        return Err(RpcErr::InvalidPayloadAttributes(
            "forkChoiceV2 with Beacon Root".to_string(),
        ));
    }
    if !chain_config.is_shanghai_activated(attributes.timestamp) {
        return Err(RpcErr::UnsuportedFork(
            "forkChoiceV2 used to build pre-Shanghai payload".to_string(),
        ));
    }
    if chain_config.is_cancun_activated(attributes.timestamp) {
        return Err(RpcErr::UnsuportedFork(
            "forkChoiceV2 used to build Cancun payload".to_string(),
        ));
    }
    if attributes.timestamp <= head_block.timestamp {
        return Err(RpcErr::InvalidPayloadAttributes(
            "invalid timestamp".to_string(),
        ));
    }
    Ok(())
}

fn build_payload(
    attributes: &PayloadAttributes,
    context: RpcApiContext,
    fork_choice_state: &ForkChoiceState,
    version: u8,
) -> Result<u64, RpcErr> {
    info!("Fork choice updated includes payload attributes. Creating a new payload.");
    let args = BuildPayloadArgs {
        parent: fork_choice_state.head_block_hash,
        timestamp: attributes.timestamp,
        fee_recipient: attributes.suggested_fee_recipient,
        random: attributes.prev_randao,
        withdrawals: attributes.withdrawals.clone(),
        beacon_root: attributes.parent_beacon_block_root,
        version,
    };
    let payload_id = args.id();
    let payload = match create_payload(&args, &context.storage) {
        Ok(payload) => payload,
        Err(ChainError::EvmError(error)) => return Err(error.into()),
        // Parent block is guaranteed to be present at this point,
        // so the only errors that may be returned are internal storage errors
        Err(error) => return Err(RpcErr::Internal(error.to_string())),
    };
    context.storage.add_payload(payload_id, payload)?;

    Ok(payload_id)
}