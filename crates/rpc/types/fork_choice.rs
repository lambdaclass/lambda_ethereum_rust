use ethereum_rust_core::{types::Withdrawal, Address, H256, U256};
use serde::Deserialize;

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
    timestamp: U256,
    prev_randao: H256,
    suggested_fee_recipient: Address,
    withdrawals: Vec<Withdrawal>,
    parent_beacon_block_root: H256,
}
