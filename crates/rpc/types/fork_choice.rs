use ethereum_rust_core::{types::Withdrawal, Address, H256, U256};
use serde::{Deserialize, Serialize};

use super::payload::PayloadStatus;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForkChoiceState {
    #[allow(unused)]
    pub head_block_hash: H256,
    pub safe_block_hash: H256,
    pub finalized_block_hash: H256,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(unused)]
pub struct PayloadAttributesV3 {
    pub timestamp: U256,
    pub prev_randao: H256,
    pub suggested_fee_recipient: Address,
    pub withdrawals: Vec<Withdrawal>,
    pub parent_beacon_block_root: H256,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForkChoiceResponse {
    pub payload_status: PayloadStatus,
    pub payload_id: Option<u8>,
}

impl ForkChoiceResponse {
    pub fn set_id(&mut self, id: u8) {
        self.payload_id = Some(id)
    }
}

impl From<PayloadStatus> for ForkChoiceResponse {
    fn from(value: PayloadStatus) -> Self {
        Self {
            payload_status: value,
            payload_id: None,
        }
    }
}
