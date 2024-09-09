use ethereum_rust_core::{serde_utils, H256};
use ethereum_rust_storage::Store;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, warn};

use crate::utils::RpcErr;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeTransitionConfigPayload {
    #[serde(with = "serde_utils::u128::hex_str")]
    terminal_total_difficulty: u128,
    terminal_block_hash: H256,
    #[serde(with = "serde_utils::u64::hex_str")]
    terminal_block_number: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeTransitionConfigV1Req {
    payload: ExchangeTransitionConfigPayload,
}

impl std::fmt::Display for ExchangeTransitionConfigV1Req {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ExchangeTransitionConfigV1Req {{ terminal_total_difficulty: {}, terminal_block_hash: {:?}, terminal_block_number: {} }}",
            self.payload.terminal_total_difficulty,
            self.payload.terminal_block_hash,
            self.payload.terminal_block_number
        )
    }
}

impl ExchangeTransitionConfigV1Req {
    pub fn parse(params: &Option<Vec<Value>>) -> Option<ExchangeTransitionConfigV1Req> {
        let params = params.as_ref()?;
        if params.len() != 1 {
            return None;
        };
        let params: ExchangeTransitionConfigPayload =
            serde_json::from_value(params[0].clone()).unwrap();
        Some(ExchangeTransitionConfigV1Req {
            payload: ExchangeTransitionConfigPayload {
                terminal_total_difficulty: params.terminal_total_difficulty,
                terminal_block_hash: params.terminal_block_hash,
                terminal_block_number: params.terminal_block_number,
            },
        })
    }
}

/// Deprecated endpoint used before the merge, though still needed for certain hive tests
/// ExchangeTransitionConfigurationV1 checks the given configuration against the configuration of the node.
pub fn exchange_transition_config_v1(
    req: ExchangeTransitionConfigV1Req,
    storage: Store,
) -> Result<Value, RpcErr> {
    info!("Received new engine request: {req}");
    let payload = req.payload;

    let chain_config = storage.get_chain_config()?.ok_or(RpcErr::Internal)?;
    let terminal_total_difficulty = chain_config.terminal_total_difficulty;

    if terminal_total_difficulty.unwrap_or_default() != payload.terminal_total_difficulty {
        warn!(
            "Invalid terminal total difficulty configured: execution {:?} consensus {}",
            terminal_total_difficulty, payload.terminal_total_difficulty
        );
    };

    let block = storage.get_block_header(payload.terminal_block_number)?;
    let terminal_block_hash = block.map_or(H256::zero(), |block| block.compute_block_hash());

    serde_json::to_value(ExchangeTransitionConfigPayload {
        terminal_block_hash,
        terminal_block_number: payload.terminal_block_number,
        terminal_total_difficulty: terminal_total_difficulty.unwrap_or_default(),
    })
    .map_err(|_| RpcErr::Internal)
}
