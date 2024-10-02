use std::u64;

use ethereum_types::{Address, H32, U256};

type AccessList = Vec<(Address, Vec<U256>)>;
type VersionedHash = H32;

/// Transaction destination.
#[derive(Clone, Debug)]
pub enum TransactTo {
    /// Simple call to an address.
    Call(Address),
    /// Contract creation.
    Create,
}

#[derive(Debug, Clone)]
pub enum Transaction {
    Legacy {
        sender: Address,
        chain_id: u64,
        nonce: U256,
        gas_limit: u64,
        to: TransactTo,
        value: U256,
        gas_price: u64,
    },
    AccessList {
        sender: Address,
        chain_id: u64,
        nonce: U256,
        gas_limit: u64,
        to: TransactTo,
        value: U256,
        gas_price: u64,
        access_list: AccessList,
    },
    FeeMarket {
        sender: Address,
        chain_id: u64,
        nonce: U256,
        gas_limit: u64,
        to: TransactTo,
        value: U256,
        max_fee_per_gas: u64,
        max_priority_fee_per_gas: u64,
        access_list: AccessList,
    },
    Blob {
        sender: Address,
        chain_id: u64,
        nonce: U256,
        gas_limit: u64,
        to: Address, // must not be null
        value: U256,
        max_fee_per_gas: u64,
        max_priority_fee_per_gas: u64,
        access_list: AccessList,
        max_fee_per_blob_gas: U256,
        blob_versioned_hashes: Vec<VersionedHash>,
    },
}

impl Default for Transaction {
    fn default() -> Self {
        Self::Legacy {
            sender: Address::default(),
            chain_id: Default::default(),
            nonce: Default::default(),
            gas_limit: u64::MAX,
            to: TransactTo::Create,
            value: Default::default(),
            gas_price: Default::default(),
        }
    }
}
