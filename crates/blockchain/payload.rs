use std::cmp::min;

use ethereum_rust_core::{
    types::{
        calculate_base_fee_per_gas, compute_receipts_root, compute_transactions_root,
        compute_withdrawals_root, Block, BlockBody, BlockHash, BlockHeader, Withdrawal,
        DEFAULT_OMMERS_HASH,
    },
    Address, Bloom, Bytes, H256, U256,
};
use ethereum_rust_evm::{
    beacon_root_contract_call, evm_state, get_state_transitions, process_withdrawals, spec_id,
    SpecId,
};
use ethereum_rust_rlp::encode::RLPEncode;
use ethereum_rust_storage::Store;
use sha3::{Digest, Keccak256};

use crate::{
    constants::{GAS_LIMIT_BOUND_DIVISOR, MIN_GAS_LIMIT, TARGET_BLOB_GAS_PER_BLOCK},
    error::ChainError,
};

pub struct BuildPayloadArgs {
    pub parent: BlockHash,
    pub timestamp: u64,
    pub fee_recipient: Address,
    pub random: H256,
    pub withdrawals: Vec<Withdrawal>,
    pub beacon_root: Option<H256>,
    pub version: u8,
}

impl BuildPayloadArgs {
    /// Computes an 8-byte identifier by hashing the components of the payload arguments.
    pub fn id(&self) -> u64 {
        let mut hasher = Keccak256::new();
        hasher.update(self.parent);
        hasher.update(self.timestamp.to_be_bytes());
        hasher.update(self.random);
        hasher.update(self.fee_recipient);
        hasher.update(self.withdrawals.encode_to_vec());
        if let Some(beacon_root) = self.beacon_root {
            hasher.update(beacon_root);
        }
        let res = &mut hasher.finalize()[..8];
        res[0] = self.version;
        u64::from_be_bytes(res.try_into().unwrap())
    }
}

/// Builds a new payload based on the payload arguments
// Basic payload block building, can and should be improved
pub fn build_payload(args: &BuildPayloadArgs, storage: &Store) -> Result<Block, ChainError> {
    // TODO: check where we should get builder values from
    const DEFAULT_BUILDER_GAS_CEIL: u64 = 30_000_000;
    let parent_block = storage
        .get_block_header_by_hash(args.parent)?
        .ok_or_else(|| ChainError::ParentNotFound)?;
    let chain_config = storage.get_chain_config()?;
    let gas_limit = calc_gas_limit(parent_block.gas_limit, DEFAULT_BUILDER_GAS_CEIL);
    let mut payload = Block {
        header: BlockHeader {
            parent_hash: args.parent,
            ommers_hash: *DEFAULT_OMMERS_HASH,
            coinbase: args.fee_recipient,
            state_root: parent_block.state_root,
            transactions_root: compute_transactions_root(&[]),
            receipts_root: compute_receipts_root(&[]),
            logs_bloom: Bloom::default(),
            difficulty: U256::zero(),
            number: parent_block.number.saturating_add(1),
            gas_limit,
            gas_used: 0,
            timestamp: args.timestamp,
            // TODO: should use builder config's extra_data
            extra_data: Bytes::new(),
            prev_randao: args.random,
            nonce: 0,
            base_fee_per_gas: calculate_base_fee_per_gas(
                gas_limit,
                parent_block.gas_limit,
                parent_block.gas_used,
                parent_block.base_fee_per_gas.unwrap_or_default(),
            ),
            withdrawals_root: chain_config
                .is_shanghai_activated(args.timestamp)
                .then_some(compute_withdrawals_root(&args.withdrawals)),
            blob_gas_used: Some(0),
            excess_blob_gas: chain_config.is_cancun_activated(args.timestamp).then_some(
                calc_excess_blob_gas(
                    parent_block.excess_blob_gas.unwrap_or_default(),
                    parent_block.blob_gas_used.unwrap_or_default(),
                ),
            ),
            parent_beacon_block_root: args.beacon_root,
        },
        // Empty body as we just created this payload
        body: BlockBody {
            transactions: Vec::new(),
            ommers: Vec::new(),
            withdrawals: Some(args.withdrawals.clone()),
        },
    };
    // Apply withdrawals & call beacon root contract, and obtain the new state root
    let spec_id = spec_id(storage, args.timestamp)?;
    let mut evm_state = evm_state(storage.clone(), parent_block.number);
    if args.beacon_root.is_some() && spec_id == SpecId::CANCUN {
        beacon_root_contract_call(&mut evm_state, &payload.header, spec_id)?;
    }
    process_withdrawals(&mut evm_state, &args.withdrawals)?;
    let account_updates = get_state_transitions(&mut evm_state);
    payload.header.state_root = storage
        .apply_account_updates(parent_block.number, &account_updates)?
        .unwrap_or_default();
    Ok(payload)
}

fn calc_gas_limit(parent_gas_limit: u64, desired_limit: u64) -> u64 {
    let delta = parent_gas_limit / GAS_LIMIT_BOUND_DIVISOR - 1;
    let mut limit = parent_gas_limit;
    let desired_limit = min(desired_limit, MIN_GAS_LIMIT);
    if limit < desired_limit {
        limit = parent_gas_limit + delta;
        if limit > desired_limit {
            limit = desired_limit
        }
        return limit;
    }
    if limit > desired_limit {
        limit = parent_gas_limit - delta;
        if limit < desired_limit {
            limit = desired_limit
        }
    }
    limit
}

fn calc_excess_blob_gas(parent_excess_blob_gas: u64, parent_blob_gas_used: u64) -> u64 {
    let excess_blob_gas = parent_excess_blob_gas + parent_blob_gas_used;
    if excess_blob_gas < TARGET_BLOB_GAS_PER_BLOCK {
        0
    } else {
        excess_blob_gas - TARGET_BLOB_GAS_PER_BLOCK
    }
}

/// Calculates the total fees paid by the payload block
/// Only potential errors are storage errors which should be treated as internal errors by rpc providers
pub fn payload_block_value(block: &Block, storage: &Store) -> Option<U256> {
    let mut total_fee = U256::zero();
    let mut last_cummulative_gas_used = 0;
    for (index, tx) in block.body.transactions.iter().enumerate() {
        // Execution already succeded by this point so we can asume the fee is valid
        let fee = tx.effective_gas_tip(block.header.base_fee_per_gas)?;
        let receipt = storage
            .get_receipt(block.header.number, index as u64)
            .ok()??;
        total_fee += U256::from(fee) * (receipt.cumulative_gas_used - last_cummulative_gas_used);
        last_cummulative_gas_used = receipt.cumulative_gas_used;
    }
    Some(total_fee)
}
