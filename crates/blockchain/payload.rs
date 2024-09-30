use std::{
    cmp::{min, Ordering},
    collections::HashMap,
};

use ethereum_rust_core::{
    types::{
        calculate_base_fee_per_blob_gas, calculate_base_fee_per_gas, compute_receipts_root,
        compute_transactions_root, compute_withdrawals_root, Block, BlockBody, BlockHash,
        BlockHeader, Receipt, Transaction, Withdrawal, DEFAULT_OMMERS_HASH,
    },
    Address, Bloom, Bytes, H256, U256,
};
use ethereum_rust_evm::{
    beacon_root_contract_call, evm_state, execute_tx, get_state_transitions, process_withdrawals,
    spec_id, EvmState, SpecId,
};
use ethereum_rust_rlp::encode::RLPEncode;
use ethereum_rust_storage::Store;
use sha3::{Digest, Keccak256};

use crate::{
    constants::{
        GAS_LIMIT_BOUND_DIVISOR, MAX_BLOB_GAS_PER_BLOCK, MIN_GAS_LIMIT, TARGET_BLOB_GAS_PER_BLOCK,
        TX_GAS_COST,
    },
    error::ChainError,
    mempool::{self, PendingTxFilter},
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

/// Creates a new payload based on the payload arguments
// Basic payload block building, can and should be improved
pub fn create_payload(args: &BuildPayloadArgs, storage: &Store) -> Result<Block, ChainError> {
    // TODO: check where we should get builder values from
    const DEFAULT_BUILDER_GAS_CEIL: u64 = 30_000_000;
    let parent_block = storage
        .get_block_header_by_hash(args.parent)?
        .ok_or_else(|| ChainError::ParentNotFound)?;
    let chain_config = storage.get_chain_config()?;
    let gas_limit = calc_gas_limit(parent_block.gas_limit, DEFAULT_BUILDER_GAS_CEIL);
    let payload = Block {
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
    // // Apply withdrawals & call beacon root contract, and obtain the new state root
    // let spec_id = spec_id(storage, args.timestamp)?;
    // let mut evm_state = evm_state(storage.clone(), parent_block.number);
    // if args.beacon_root.is_some() && spec_id == SpecId::CANCUN {
    //     beacon_root_contract_call(&mut evm_state, &payload.header, spec_id)?;
    // }
    // process_withdrawals(&mut evm_state, &args.withdrawals)?;
    // let account_updates = get_state_transitions(&mut evm_state);
    // payload.header.state_root = storage
    //     .apply_account_updates(parent_block.number, &account_updates)?
    //     .unwrap_or_default();
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

/// Completes the payload building process, return the block value
pub fn build_payload(payload: &mut Block, store: &Store) -> Result<U256, ChainError> {
    // Apply withdrawals & call beacon root contract, and obtain the new state root
    let parent_number = payload.header.number.saturating_sub(1);
    let spec_id = spec_id(store, payload.header.timestamp)?;
    let mut evm_state = evm_state(store.clone(), parent_number);
    if payload.header.parent_beacon_block_root.is_some() && spec_id == SpecId::CANCUN {
        beacon_root_contract_call(&mut evm_state, &payload.header, spec_id)?;
    }
    let withdrawals = payload.body.withdrawals.clone().unwrap_or_default();
    process_withdrawals(&mut evm_state, &withdrawals)?;
    fill_transactions(payload, &mut evm_state)
}

/// Fills the payload with transactions taken from the mempool
/// Returns the block value
pub fn fill_transactions(
    payload_block: &mut Block,
    evm_state: &mut EvmState,
) -> Result<U256, ChainError> {
    let chain_config = evm_state.database().get_chain_config()?;
    let base_fee_per_blob_gas = U256::from(calculate_base_fee_per_blob_gas(
        payload_block.header.excess_blob_gas.unwrap_or_default(),
    ));
    let tx_filter = PendingTxFilter {
        /*TODO: add tip filter */
        base_fee: payload_block.header.base_fee_per_gas,
        blob_fee: Some(base_fee_per_blob_gas),
        ..Default::default()
    };
    let plain_tx_filter = PendingTxFilter {
        only_plain_txs: true,
        ..tx_filter
    };
    let blob_tx_filter = PendingTxFilter {
        only_blob_txs: true,
        ..tx_filter
    };
    let mut plain_txs = TransactionQueue::new(
        mempool::filter_transactions(&plain_tx_filter, evm_state.database())?,
        payload_block.header.base_fee_per_gas,
    );
    let mut blob_txs = TransactionQueue::new(
        mempool::filter_transactions(&blob_tx_filter, evm_state.database())?,
        payload_block.header.base_fee_per_gas,
    );
    // Commit txs
    let mut receipts = Vec::new();
    let mut total_fee = U256::zero();
    let mut remaining_gas = payload_block.header.gas_limit;
    let blobs = 0_u64;
    loop {
        if remaining_gas < TX_GAS_COST {
            // No more gas to run transactions
            break;
        };
        if !blob_txs.is_empty()
            && base_fee_per_blob_gas * blobs > U256::from(MAX_BLOB_GAS_PER_BLOCK)
        {
            // No more blob space to run blob transactions
            blob_txs.clear();
        }
        // Fetch the next transactions
        let (head_tx, is_blob) = match (plain_txs.peek(), blob_txs.peek()) {
            (None, None) => break,
            (None, Some(tx)) => (tx, true),
            (Some(tx), None) => (tx, false),
            (Some(a), Some(b)) if compare_heads(&a, &b).is_lt() => (b, true),
            (Some(tx), _) => (tx.clone(), false),
        };
        let txs = if is_blob {
            &mut blob_txs
        } else {
            &mut plain_txs
        };
        if remaining_gas < head_tx.tx.gas_limit() {
            // We don't have enough gas left for the transaction, so we skip all txs from this account
            txs.pop();
        }
        // Pull transaction from the mempool
        // TODO: maybe fetch hash too when filtering mempool so we don't have to compute it here
        let hash = head_tx.tx.compute_hash();
        mempool::remove_transaction(hash, evm_state.database())?;

        // Check wether the tx is replay-protected
        if head_tx.tx.protected() && !chain_config.is_eip155_activated(payload_block.header.number)
        {
            // Ignore replay protected tx & all txs from the sender
            txs.pop();
        }
        // Execute tx
        let prev_remaining_gas = remaining_gas;
        let receipt =
            match apply_transaction(payload_block, &head_tx.tx, evm_state, &mut remaining_gas) {
                Ok(receipt) => {
                    txs.shift();
                    receipt
                }
                // Ignore following txs from sender
                Err(_) => {
                    txs.pop();
                    continue;
                }
            };
        total_fee += U256::from(prev_remaining_gas - remaining_gas) * head_tx.tip;
        // Add transaction to block
        payload_block.body.transactions.push(head_tx.tx);
        // Save receipt for hash calculation
        receipts.push(receipt);
    }
    // Finalize block
    let account_updates = get_state_transitions(evm_state);
    payload_block.header.state_root = evm_state
        .database()
        .apply_account_updates(
            payload_block.header.number.saturating_sub(1),
            &account_updates,
        )?
        .unwrap_or_default();
    payload_block.header.transactions_root =
        compute_transactions_root(&payload_block.body.transactions);
    payload_block.header.receipts_root = compute_receipts_root(&receipts);
    payload_block.header.gas_used = payload_block.header.gas_limit - remaining_gas;

    Ok(total_fee)
}

fn apply_transaction(
    payload_block: &mut Block,
    tx: &Transaction,
    evm_state: &mut EvmState,
    remaining_gas: &mut u64,
) -> Result<Receipt, ChainError> {
    let result = execute_tx(
        &tx,
        &payload_block.header,
        evm_state,
        spec_id(evm_state.database(), payload_block.header.timestamp)?,
    )?;
    *remaining_gas -= result.gas_used();
    let receipt = Receipt::new(
        tx.tx_type(),
        result.is_success(),
        payload_block.header.gas_limit - *remaining_gas,
        result.logs(),
    );
    Ok(receipt)
}

struct TransactionQueue {
    // The first transaction for each account along with its tip, sorted by highest tip
    heads: Vec<HeadTransaction>,
    // The remaining txs grouped by account and sorted by nonce
    txs: HashMap<Address, Vec<Transaction>>,
    base_fee: Option<u64>,
}

#[derive(Clone)]
struct HeadTransaction {
    tx: Transaction,
    sender: Address,
    tip: u64,
}

impl TransactionQueue {
    fn new(mut txs: HashMap<Address, Vec<Transaction>>, base_fee: Option<u64>) -> Self {
        let mut heads = Vec::new();
        for (address, txs) in txs.iter_mut() {
            // This should be a newly filtered tx list so we are guaranteed to have a first element
            let head_tx = txs.remove(0);
            heads.push(HeadTransaction {
                // We already ran this method when filtering the transactions from the mempool so it shouldn't fail
                tip: head_tx.effective_gas_tip(base_fee).unwrap(),
                tx: head_tx,
                sender: *address,
            });
        }
        heads.sort_by(|a, b| compare_heads(a, b));
        TransactionQueue {
            heads,
            txs,
            base_fee,
        }
    }

    fn clear(&mut self) {
        self.heads.clear();
    }

    fn is_empty(&self) -> bool {
        self.heads.is_empty()
    }

    /// Returns the head transaction with the highest tip
    /// If there is more than one transaction with the highest tip, return the one with the lowest timestamp
    fn peek(&self) -> Option<HeadTransaction> {
        self.heads.first().map(|tx| tx.clone())
    }

    /// Removes current head transaction and all transactions from the given sender
    fn pop(&mut self) {
        if !self.is_empty() {
            let sender = self.heads.remove(0).sender;
            self.txs.remove(&sender);
        }
    }

    /// Remove the top transaction
    /// Add a tx from the same sender as head transaction
    fn shift(&mut self) {
        let tx = self.heads.remove(0);
        if let Some(txs) = self.txs.get_mut(&tx.sender) {
            // Fetch next head
            if !txs.is_empty() {
                let head_tx = txs.remove(0);
                let head = HeadTransaction {
                    // We already ran this method when filtering the transactions from the mempool so it shouldn't fail
                    tip: head_tx.effective_gas_tip(self.base_fee).unwrap(),
                    tx: head_tx,
                    sender: tx.sender,
                };
                // Insert head into heads list while maintaing order
                let mut index = 0;
                loop {
                    if self
                        .heads
                        .get(index)
                        .is_some_and(|current_head| compare_heads(current_head, &head).is_gt())
                    {
                        index += 1;
                    } else {
                        self.heads.insert(index, head.clone());
                    }
                }
            }
        }
    }
}

/// Returns the order in which txs a and b should be executed
/// The transaction with the highest tip should go first,
///  if both have the same tip then the one with the lowest timestamp should go first
/// This function will not return Ordering::Equal (TODO: make this true with timestamp)
/// TODO: add timestamp
fn compare_heads(a: &HeadTransaction, b: &HeadTransaction) -> Ordering {
    b.tip.cmp(&a.tip)
    // TODO: Add timestamp field to mempool txs so we can compare by it
}
