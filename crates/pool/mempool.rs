use ethereum_rust_core::{types::Transaction, H256};
use ethereum_rust_storage::Store;
mod error;
pub use error::MempoolError;

pub fn add_transaction(transaction: Transaction, store: Store) -> Result<H256, MempoolError> {
    // Validate transaction
    validate_transaction(&transaction)?;

    let hash = transaction.compute_hash();

    // Add transaction to storage
    store.add_transaction_to_pool(hash, transaction)?;

    Ok(hash)
}

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
fn validate_transaction(_transaction: &Transaction) -> Result<(), MempoolError> {
    // TODO: Add validations here

    Ok(())
}
