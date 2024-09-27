use std::u64;

use ethereum_types::{Address, H32, U256};

type AccessList = Vec<(Address, Vec<U256>)>;
type VersionedHash = H32;

#[derive(Debug, Clone)]
pub enum Transaction {
    Legacy {
        chain_id: u64,
        nonce: U256,
        gas_limit: u64,
        to: Option<Address>,
        value: U256,
        r: U256,
        s: U256,
        gas_price: u64,
    },
    AccessList {
        chain_id: u64,
        nonce: U256,
        gas_limit: u64,
        to: Option<Address>,
        value: U256,
        r: U256,
        s: U256,
        gas_price: u64,
        access_list: AccessList,
        y_parity: U256,
    },
    FeeMarket {
        chain_id: u64,
        nonce: U256,
        gas_limit: u64,
        to: Option<Address>,
        value: U256,
        r: U256,
        s: U256,
        max_fee_per_gas: u64,
        max_priority_fee_per_gas: u64,
        access_list: AccessList,
        y_parity: U256,
    },
    Blob {
        chain_id: u64,
        nonce: U256,
        gas_limit: u64,
        to: Address, // must not be null
        value: U256,
        r: U256,
        s: U256,
        max_fee_per_gas: u64,
        max_priority_fee_per_gas: u64,
        access_list: AccessList,
        y_parity: U256,
        max_fee_per_blob_gas: U256,
        blob_versioned_hashes: Vec<VersionedHash>,
    },
}

impl Default for Transaction {
    fn default() -> Self {
        Self::Legacy {
            chain_id: Default::default(),
            nonce: Default::default(),
            gas_limit: u64::MAX,
            to: Default::default(),
            value: Default::default(),
            r: Default::default(),
            s: Default::default(),
            gas_price: Default::default(),
        }
    }
}
