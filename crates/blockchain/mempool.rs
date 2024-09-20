use crate::error::MempoolError;
use ethereum_rust_core::{
    types::{BlockHeader, ChainConfig, Transaction},
    H256, U256,
};
use ethereum_rust_storage::Store;

pub fn add_transaction(transaction: Transaction, store: Store) -> Result<H256, MempoolError> {
    // Validate transaction
    validate_transaction(&transaction, store.clone())?;

    let hash = transaction.compute_hash();

    // Add transaction to storage
    store.add_transaction_to_pool(hash, transaction)?;

    Ok(hash)
}

pub fn get_transaction(hash: H256, store: Store) -> Result<Option<Transaction>, MempoolError> {
    Ok(store.get_transaction_from_pool(hash)?)
}

// Defined in [EIP-170](https://eips.ethereum.org/EIPS/eip-170)
pub const MAX_CODE_SIZE: usize = 0x6000;
// Defined in [EIP-3860](https://eips.ethereum.org/EIPS/eip-3860)
pub const MAX_INITCODE_SIZE: usize = 2 * MAX_CODE_SIZE;

// Gas cost for each non zero byte on transaction data
pub const TX_DATA_NON_ZERO_GAS: u64 = 68;
// Gas cost for each non zero byte on transaction data, modified on [EIP-2028](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-2028.md)
pub const TX_DATA_NON_ZERO_GAS_EIP2028: u64 = 16;

//TODO: THIS ALL SHOULD BE MOVED TO chain/constants

// YELLOW PAPER CONSTANTS

/// Base gas cost for each non contract creating transaction
pub const TX_GAS_COST: u64 = 21000;

/// Base gas cost for each contract creating transaction
pub const TX_CREATE_GAS_COST: u64 = 53000;

// Gas cost for each zero byte on transaction data
pub const TX_DATA_ZERO_GAS_COST: u64 = 4;

// Gas cost for each init code word on transaction data
pub const TX_INIT_CODE_WORD_GAS_COST: u64 = 2;

// Gas cost for each init code word on transaction data
pub const TX_ACCESS_LIST_ADDRESS_GAS: u64 = 2400;

// Gas cost for each init code word on transaction data
pub const TX_ACCESS_LIST_STORAGE_KEY_GAS: u64 = 1900;

/*

Stateless validations

1. This transaction is valid on current mempool
2. Transaction's encoded size is smaller than maximum allowed
3. The transaction type (TxType) is allowed on current mempool
4. If the transaction creates a new contract, the init code should be smaller than a certain maximum
5. The transacion's value is positive
6. The block's header gas limit is higher than the transaction's gas limit
7. Sanity checks for extremly large numbers on the gas fee and gas tip
8. maxFeePerGas is greater or equal than maxPriorityFeePerGas
9. Make sure the transaction is signed properly
10. Ensure the transaction has more gas than the bare minimum needed to cover the transaction metadata, which includes:
    - Data len
    - Access lists
    - Is contract creation
11. Ensure the maxPriorityFeePerGas is high enough to cover the requirement of the calling pool (the minimum to be included in)
12. Ensure the blob fee cap satisfies the minimum blob gas price
13. Ensure a Blob Transaction comes with its sidecar:
  1. Validate number of BlobHashes is positive
  2. Validate number of BlobHashes is less than the maximum allowed per block,
     which may be computed as `maxBlobGasPerBlock / blobTxBlobGasPerBlob`
  3. Ensure number of BlobHashes is equal to:
    - The number of blobs
    - The number of commitments
    - The number of proofs
  4. Validate that the hashes matches with the commitments, performing a `kzg4844` hash.
  5. Verify the blob proofs with the `kzg4844`


Stateful validations

1. Ensure transaction nonce is higher than the `from` address stored nonce
2. Certain pools do not allow for nonce gaps. Ensure a gap is not produced (that is, the transaction nonce is exactly the following of the stored one)
3. Ensure the transactor has enough funds to cover transaction cost:
    - Transaction cost is calculated as `(gas * gasPrice) + (blobGas * blobGasPrice) + value`
 4. In case of transaction reorg, ensure the transactor has enough funds to cover for transaction replacements without overdrafts.
    - This is done by comparing the total spent gas of the transactor from all pooled transactions, and accounting for the necessary gas spenditure if any of those transactions is replaced.
 5. Ensure the transactor is able to add a new transaction. The number of transactions sent by an account may be limited by a certain configured value

*/

fn validate_transaction(tx: &Transaction, store: Store) -> Result<(), MempoolError> {
    // TODO: Add validations here

    let header_no = store
        .get_latest_block_number()?
        .ok_or(MempoolError::NoBlockHeaderError)?;
    let header = store
        .get_block_header(header_no)?
        .ok_or(MempoolError::NoBlockHeaderError)?;
    let config = store.get_chain_config()?;

    // NOTE: We could add a tx size limit here, but it's not in the actual spec

    // Check init code size
    if config.is_shanghai_activated(header.timestamp)
        && tx.is_contract_creation()
        && tx.data().len() > MAX_INITCODE_SIZE
    {
        return Err(MempoolError::TxMaxInitCodeSizeError);
    }

    // Check gas limit is less than header's gas limit
    if header.gas_limit < tx.gas_limit() {
        return Err(MempoolError::TxGasLimitExceededError);
    }

    // Check priority fee is less or equal than gas fee gap
    if tx.max_priority_fee().unwrap_or(0) > tx.max_fee_per_gas().unwrap_or(0) {
        return Err(MempoolError::TxTipAboveFeeCapError);
    }

    // Check that the gas limit is covers the gas needs for transaction metadata.
    if tx.gas_limit() < transaction_intrinsic_gas(tx, &header, &config)? {
        return Err(MempoolError::TxIntrinsicGasCostAboveLimitError);
    }

    Ok(())
}

fn transaction_intrinsic_gas(
    tx: &Transaction,
    header: &BlockHeader,
    config: &ChainConfig,
) -> Result<u64, MempoolError> {
    let is_contract_creation = tx.is_contract_creation();

    let mut gas = if is_contract_creation {
        TX_CREATE_GAS_COST
    } else {
        TX_GAS_COST
    };

    let data_len = tx.data().len() as u64;

    if data_len > 0 {
        let non_zero_gas_cost = if config.is_istanbul_activated(header.number) {
            TX_DATA_NON_ZERO_GAS_EIP2028
        } else {
            TX_DATA_NON_ZERO_GAS
        };

        let non_zero_count = tx.data().iter().filter(|&&x| x != 0u8).count() as u64;

        gas = gas
            .checked_add(non_zero_count * non_zero_gas_cost)
            .ok_or(MempoolError::TxGasOverflowError)?;

        let zero_count = data_len - non_zero_count;

        gas = gas
            .checked_add(zero_count * TX_DATA_ZERO_GAS_COST)
            .ok_or(MempoolError::TxGasOverflowError)?;

        if is_contract_creation && config.is_shanghai_activated(header.timestamp) {
            // Len in 32 bytes sized words
            let len_in_words = data_len.saturating_add(31) / 32;

            gas = gas
                .checked_add(len_in_words * TX_INIT_CODE_WORD_GAS_COST)
                .ok_or(MempoolError::TxGasOverflowError)?;
        }
    }

    let storage_keys_count: u64 = tx
        .access_list()
        .iter()
        .map(|(_, keys)| keys.len() as u64)
        .sum();

    gas = gas
        .checked_add(tx.access_list().len() as u64 * TX_ACCESS_LIST_ADDRESS_GAS)
        .ok_or(MempoolError::TxGasOverflowError)?;

    gas = gas
        .checked_add(storage_keys_count * TX_ACCESS_LIST_STORAGE_KEY_GAS)
        .ok_or(MempoolError::TxGasOverflowError)?;

    Ok(gas)
}
