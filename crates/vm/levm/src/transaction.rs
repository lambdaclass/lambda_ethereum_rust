use bytes::Bytes;
use ethereum_types::{Address, H256, U256};

type AccessList = Vec<(Address, Vec<U256>)>;
// type VersionedHash = H32;

/// Transaction destination.
#[derive(Clone, Debug, Default)]
pub enum TransactTo {
    /// Simple call to an address.
    Call(Address),
    /// Contract creation.
    #[default]
    Create,
}

/// The transaction environment.
#[derive(Clone, Debug, Default)]
pub struct TxEnv {
    /// Caller aka Author aka transaction signer.
    pub caller: Address,

    /// Maximum amount of gas that should be used in executing
    /// this transaction.
    pub gas_limit: u64,

    /// The amount of wei the transaction signer will pay per unit
    /// of gas consumed during the transaction's execution.
    ///
    /// It is calculated as follows:
    ///
    /// - For type 0 and type 1 transactions:
    ///
    /// ```p = tx.gas_price```
    ///
    /// - For type 2 and type 3 transactions:
    ///
    /// ```p = gas_priority_fee + block.base_fee_per_gas```
    pub effective_gas_price: U256,

    /// The amount of wei the block's beneficiary address will
    /// receive per unit of gas consumed during the transaction's
    /// execution.
    ///
    /// If is calculated as follows:
    ///
    /// - For type 0 and type 1 transactions:
    ///
    /// ```f = tx.gas_price - block.base_fee_per_gas```
    ///
    /// - For type 2 and type 3 transactions:
    ///
    /// ```f = min(tx.max_priority_fee_per_gas, tx.max_fee_per_gas - tx.base_fee_per_gas)```
    pub priority_fee: U256,

    /// The destination of the transaction.
    pub transact_to: TransactTo,
    /// The value sent to `transact_to`.
    pub value: U256,
    // The data of the transaction.
    pub data: Bytes,
    // The nonce of the transaction.
    pub nonce: u64,
    // The chain ID of the transaction. If set to `None`, no checks are performed.
    //
    // Incorporated as part of the Spurious Dragon upgrade via [EIP-155].
    //
    // [EIP-155]: https://eips.ethereum.org/EIPS/eip-155
    pub chain_id: Option<u64>,
    // A list of addresses and storage keys that the transaction plans to access.
    //
    // Added in [EIP-2930].
    //
    // [EIP-2930]: https://eips.ethereum.org/EIPS/eip-2930
    pub access_list: Option<AccessList>,

    // The list of blob versioned hashes. Per EIP there should be at least
    // one blob present if [`Self::max_fee_per_blob_gas`] is `Some`.
    //
    // Incorporated as part of the Cancun upgrade via [EIP-4844].
    //
    // [EIP-4844]: https://eips.ethereum.org/EIPS/eip-4844
    pub blob_hashes: Vec<H256>,
    // The max fee per blob gas.
    //
    // Incorporated as part of the Cancun upgrade via [EIP-4844].
    //
    // [EIP-4844]: https://eips.ethereum.org/EIPS/eip-4844
    pub max_fee_per_blob_gas: Option<U256>,
}
