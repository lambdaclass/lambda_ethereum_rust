use bytes::Bytes;
use ethereum_types::{Address, H256, U256};

use crate::rlp::encode::RLPEncode;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Transaction {
    LegacyTransaction(LegacyTransaction),
    EIP1559Transaction(EIP1559Transaction),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyTransaction {
    nonce: U256,
    gas_price: u64,
    gas: u64,
    to: Address,
    value: U256,
    data: Bytes,
    v: U256,
    r: U256,
    s: U256,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EIP1559Transaction {
    chain_id: u64,
    signer_nonce: U256,
    max_priority_fee_per_gas: u64,
    max_fee_per_gas: u64,
    gas_limit: u64,
    destination: Address,
    amount: u64,
    payload: Bytes,
    access_list: Vec<(Address, Vec<H256>)>,
    signature_y_parity: bool,
    signature_r: U256,
    signature_s: U256,
}

impl RLPEncode for Transaction {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        match self {
            Transaction::LegacyTransaction(t) => t.encode(buf),
            Transaction::EIP1559Transaction(t) => t.encode(buf),
        };
    }
}

impl RLPEncode for LegacyTransaction {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        self.nonce.encode(buf);
        self.gas_price.encode(buf);
        self.gas.encode(buf);
        self.to.encode(buf);
        self.value.encode(buf);
        self.data.encode(buf);
        self.v.encode(buf);
        self.r.encode(buf);
        self.s.encode(buf);
    }
}

impl RLPEncode for EIP1559Transaction {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        self.chain_id.encode(buf);
        self.signer_nonce.encode(buf);
        self.max_priority_fee_per_gas.encode(buf);
        self.max_fee_per_gas.encode(buf);
        self.gas_limit.encode(buf);
        self.destination.encode(buf);
        self.amount.encode(buf);
        self.payload.encode(buf);
        self.access_list.encode(buf);
        self.signature_y_parity.encode(buf);
        self.signature_r.encode(buf);
        self.signature_s.encode(buf);
    }
}

impl Transaction {
    pub fn sender(&self) -> Address {
        match self {
            Transaction::LegacyTransaction(_tx) => todo!(),
            Transaction::EIP1559Transaction(_tx) => todo!(),
        }
    }

    pub fn gas_limit(&self) -> u64 {
        match self {
            Transaction::LegacyTransaction(_tx) => todo!(),
            Transaction::EIP1559Transaction(tx) => tx.gas_limit,
        }
    }

    pub fn gas_price(&self) -> u64 {
        match self {
            Transaction::LegacyTransaction(tx) => tx.gas_price,
            Transaction::EIP1559Transaction(_tx) => todo!(),
        }
    }

    pub fn to(&self) -> Address {
        match self {
            Transaction::LegacyTransaction(tx) => tx.to,
            Transaction::EIP1559Transaction(tx) => tx.destination,
        }
    }

    pub fn value(&self) -> U256 {
        match self {
            Transaction::LegacyTransaction(tx) => tx.value,
            Transaction::EIP1559Transaction(_tx) => todo!(),
        }
    }

    pub fn max_priority_fee(&self) -> Option<u64> {
        match self {
            Transaction::LegacyTransaction(_tx) => None,
            Transaction::EIP1559Transaction(tx) => Some(tx.max_priority_fee_per_gas),
        }
    }

    pub fn chain_id(&self) -> Option<u64> {
        match self {
            Transaction::LegacyTransaction(_tx) => None,
            Transaction::EIP1559Transaction(tx) => Some(tx.chain_id),
        }
    }

    pub fn access_list(&self) -> Vec<(Address, Vec<H256>)> {
        match self {
            Transaction::LegacyTransaction(_tx) => Vec::new(),
            Transaction::EIP1559Transaction(tx) => tx.access_list.clone(),
        }
    }
}
