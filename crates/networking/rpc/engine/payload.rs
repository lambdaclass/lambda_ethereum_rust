use ethereum_rust_blockchain::error::ChainError;
use ethereum_rust_blockchain::payload::build_payload;
use ethereum_rust_blockchain::{add_block, latest_valid_hash};
use ethereum_rust_core::types::Fork;
use ethereum_rust_core::H256;
use ethereum_rust_storage::Store;
use serde_json::Value;
use tracing::{info, warn};

use crate::types::payload::ExecutionPayloadResponse;
use crate::{
    types::payload::{ExecutionPayloadV3, PayloadStatus},
    RpcErr, RpcHandler,
};

pub struct NewPayloadV3Request {
    pub payload: ExecutionPayloadV3,
    pub expected_blob_versioned_hashes: Vec<H256>,
    pub parent_beacon_block_root: H256,
}

pub struct GetPayloadV3Request {
    pub payload_id: u64,
}

impl RpcHandler for NewPayloadV3Request {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let params = params
            .as_ref()
            .ok_or(RpcErr::BadParams("No params provided".to_owned()))?;
        if params.len() != 3 {
            return Err(RpcErr::BadParams("Expected 3 params".to_owned()));
        }
        Ok(NewPayloadV3Request {
            payload: serde_json::from_value(params[0].clone())?,
            expected_blob_versioned_hashes: serde_json::from_value(params[1].clone())?,
            parent_beacon_block_root: serde_json::from_value(params[2].clone())?,
        })
    }

    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        let block_hash = self.payload.block_hash;
        info!("Received new payload with block hash: {block_hash}");

        let block = match self
            .payload
            .clone()
            .into_block(self.parent_beacon_block_root)
        {
            Ok(block) => block,
            Err(error) => {
                let result = PayloadStatus::invalid_with_err(&error.to_string());
                return serde_json::to_value(result)
                    .map_err(|error| RpcErr::Internal(error.to_string()));
            }
        };

        // Payload Validation

        // Check timestamp does not fall within the time frame of the Cancun fork
        let chain_config = storage.get_chain_config()?;
        let current_fork = chain_config.get_fork(block.header.timestamp);
        if current_fork < Fork::Cancun {
            return Err(RpcErr::UnsuportedFork(format!("{current_fork:?}")));
        }

        // Check that block_hash is valid
        let actual_block_hash = block.header.compute_block_hash();
        if block_hash != actual_block_hash {
            let result = PayloadStatus::invalid_with_err("Invalid block hash");
            return serde_json::to_value(result)
                .map_err(|error| RpcErr::Internal(error.to_string()));
        }
        info!("Block hash {block_hash} is valid");
        // Concatenate blob versioned hashes lists (tx.blob_versioned_hashes) of each blob transaction included in the payload, respecting the order of inclusion
        // and check that the resulting array matches expected_blob_versioned_hashes
        let blob_versioned_hashes: Vec<H256> = block
            .body
            .transactions
            .iter()
            .flat_map(|tx| tx.blob_versioned_hashes())
            .collect();
        if self.expected_blob_versioned_hashes != blob_versioned_hashes {
            let result = PayloadStatus::invalid_with_err("Invalid blob_versioned_hashes");
            return serde_json::to_value(result)
                .map_err(|error| RpcErr::Internal(error.to_string()));
        }
        // Check that the incoming block extends the current chain
        let last_block_number = storage.get_latest_block_number()?.ok_or(RpcErr::Internal(
            "Could not get latest block number".to_owned(),
        ))?;
        if block.header.number <= last_block_number {
            // Check if we already have this block stored
            if storage
                .get_block_number(block_hash)
                .map_err(|error| RpcErr::Internal(error.to_string()))?
                .is_some_and(|num| num == block.header.number)
            {
                let result = PayloadStatus::valid_with_hash(block_hash);
                return serde_json::to_value(result)
                    .map_err(|error| RpcErr::Internal(error.to_string()));
            }
            warn!("Should start reorg but it is not supported yet");
            return Err(RpcErr::Internal(
                "Block reorg is not supported yet".to_owned(),
            ));
        } else if block.header.number != last_block_number + 1 {
            let result = PayloadStatus::syncing();
            return serde_json::to_value(result)
                .map_err(|error| RpcErr::Internal(error.to_string()));
        }

        let latest_valid_hash =
            latest_valid_hash(&storage).map_err(|error| RpcErr::Internal(error.to_string()))?;

        // Execute and store the block
        info!("Executing payload with block hash: {block_hash}");
        let payload_status = match add_block(&block, &storage) {
            Err(ChainError::NonCanonicalParent) => Ok(PayloadStatus::syncing()),
            Err(ChainError::ParentNotFound) => Ok(PayloadStatus::invalid_with_err(
                "Could not reference parent block with parent_hash",
            )),
            Err(ChainError::InvalidBlock(error)) => {
                warn!("Error adding block: {error}");
                Ok(PayloadStatus::invalid_with(
                    latest_valid_hash,
                    error.to_string(),
                ))
            }
            Err(ChainError::EvmError(error)) => {
                warn!("Error executing block: {error}");
                Ok(PayloadStatus::invalid_with_err(&error.to_string()))
            }
            Err(ChainError::StoreError(error)) => {
                warn!("Error storing block: {error}");
                Err(RpcErr::Internal(error.to_string()))
            }
            Ok(()) => {
                info!("Block with hash {block_hash} executed succesfully");
                // TODO: We don't have a way to fetch blocks by number if they are not canonical
                // so we need to set it as canonical in order to run basic test suites
                // We should remove this line once the issue is solved
                storage.set_canonical_block(block.header.number, block_hash)?;
                info!("Block with hash {block_hash} added to storage");

                Ok(PayloadStatus::valid_with_hash(block_hash))
            }
        }?;

        serde_json::to_value(payload_status).map_err(|error| RpcErr::Internal(error.to_string()))
    }
}

impl RpcHandler for GetPayloadV3Request {
    fn parse(params: &Option<Vec<Value>>) -> Result<Self, RpcErr> {
        let params = params
            .as_ref()
            .ok_or(RpcErr::BadParams("No params provided".to_owned()))?;
        if params.len() != 1 {
            return Err(RpcErr::BadParams("Expected 1 param".to_owned()));
        };
        let Ok(hex_str) = serde_json::from_value::<String>(params[0].clone()) else {
            return Err(RpcErr::BadParams(
                "Expected param to be a string".to_owned(),
            ));
        };
        // Check that the hex string is 0x prefixed
        let Some(hex_str) = hex_str.strip_prefix("0x") else {
            return Err(RpcErr::BadHexFormat(0));
        };
        // Parse hex string
        let Ok(payload_id) = u64::from_str_radix(hex_str, 16) else {
            return Err(RpcErr::BadHexFormat(0));
        };
        Ok(GetPayloadV3Request { payload_id })
    }

    fn handle(&self, storage: Store) -> Result<Value, RpcErr> {
        info!("Requested payload with id: {:#018x}", self.payload_id);
        let Some(mut payload) = storage.get_payload(self.payload_id)? else {
            return Err(RpcErr::UnknownPayload(format!(
                "Payload with id {:#018x} not found",
                self.payload_id
            )));
        };
        let block_value = build_payload(&mut payload, &storage)
            .map_err(|error| RpcErr::Internal(error.to_string()))?;
        serde_json::to_value(ExecutionPayloadResponse::new(
            ExecutionPayloadV3::from_block(payload),
            block_value,
        ))
        .map_err(|error| RpcErr::Internal(error.to_string()))
    }
}
