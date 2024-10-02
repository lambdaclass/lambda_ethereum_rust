use std::u64;

use crate::primitives::{Address, H32, U256, Bytes};

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
