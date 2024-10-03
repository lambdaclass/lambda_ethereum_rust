use std::u64;

use crate::primitives::{Address, H32, U256, Bytes, H256};

type AccessList = Vec<(Address, Vec<U256>)>;
type VersionedHash = H32;

#[derive(Debug, Clone)]
pub enum Transaction {
    Legacy {
        chain_id: u64,
        nonce: U256,
        gas_limit: u64,
        msg_sender: Address,
        to: Option<Address>,
        value: U256,
        gas_price: u64,
        data: Bytes,
    },
    AccessList {
        chain_id: u64,
        nonce: U256,
        gas_limit: u64,
        msg_sender: Address,
        to: Option<Address>,
        value: U256,
        gas_price: u64,
        access_list: AccessList,
        y_parity: U256,
        data: Bytes,
    },
    FeeMarket {
        chain_id: u64,
        nonce: U256,
        gas_limit: u64,
        msg_sender: Address,
        to: Option<Address>,
        value: U256,
        max_fee_per_gas: u64,
        max_priority_fee_per_gas: u64,
        access_list: AccessList,
        y_parity: U256,
        data: Bytes,
    },
    Blob {
        chain_id: u64,
        nonce: U256,
        gas_limit: u64,
        msg_sender: Address,
        to: Address, // must not be null
        value: U256,
        max_fee_per_gas: u64,
        max_priority_fee_per_gas: u64,
        access_list: AccessList,
        y_parity: U256,
        max_fee_per_blob_gas: U256,
        blob_versioned_hashes: Vec<VersionedHash>,
        data: Bytes,
    },
}

// pub struct TransactionParts {
//     pub data: Vec<Bytes>,
//     pub gas_limit: Vec<U256>,
//     pub gas_price: Option<U256>,
//     pub nonce: U256,
//     pub secret_key: H256,
//     /// if sender is not present we need to derive it from secret key.
//     #[serde(default)]
//     pub sender: Option<Address>,
//     #[serde(deserialize_with = "deserialize_maybe_empty")]
//     pub to: Option<Address>,
//     pub value: Vec<U256>,
//     pub max_fee_per_gas: Option<U256>,
//     pub max_priority_fee_per_gas: Option<U256>,
//     #[serde(default)]
//     pub access_lists: Vec<Option<AccessList>>,
//     #[serde(default)]
//     pub blob_versioned_hashes: Vec<H256>,
//     pub max_fee_per_blob_gas: Option<U256>,
// }

impl Default for Transaction {
    fn default() -> Self {
        Self::Legacy {
            chain_id: Default::default(),
            nonce: Default::default(),
            gas_limit: u64::MAX,
            to: Default::default(),
            value: Default::default(),
            msg_sender: Default::default(),
            gas_price: Default::default(),
            data: Default::default(),
        }
    }
}
/// Transaction destination.
#[derive(Clone, Debug)]
pub enum TransactTo {
    /// Simple call to an address.
    Call(Address),
    /// Contract creation.
    Create,
}

impl Default for TransactTo {
    fn default() -> Self {
        TransactTo::Create
    }
}

/// The transaction environment.
#[derive(Clone, Debug)]
pub struct TxEnv {
    /// Caller aka Author aka transaction signer.
    pub msg_sender: Address,
    /// The gas limit of the transaction.
    pub gas_limit: u64,
    /// The gas price of the transaction.
    pub gas_price: U256,
    /// The destination of the transaction.
    pub transact_to: TransactTo,
    /// The value sent to `transact_to`.
    pub value: U256,
    // The data of the transaction.
    pub data: Bytes,
    // The nonce of the transaction.
    pub nonce: Option<u64>,
    // Caution: If set to `None`, then nonce validation against the account's nonce is skipped: [InvalidTransaction::NonceTooHigh] and [InvalidTransaction::NonceTooLow]

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
    pub access_list: AccessList,
     /// The priority fee per gas.
    ///
    /// Incorporated as part of the London upgrade via [EIP-1559].
    ///
    /// [EIP-1559]: https://eips.ethereum.org/EIPS/eip-1559
    /// aka `max_priority_fee_per_gas` or _miner tip_
    pub max_priority_fee_per_gas: Option<U256>,
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
