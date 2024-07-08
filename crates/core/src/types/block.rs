use crate::{
    rlp::{encode::RLPEncode, structs::Encoder},
    types::Receipt,
    Address, H256, U256,
};
use bytes::Bytes;
use keccak_hash::keccak;
use patricia_merkle_tree::PatriciaMerkleTree;
use serde::Deserialize;
use sha3::Keccak256;

use super::Transaction;

pub type BlockNumber = u64;
pub type Bloom = [u8; 256];

use lazy_static::lazy_static;

lazy_static! {
    pub static ref DEFAULT_OMMERS_HASH: H256 = H256::from_slice(&hex::decode("1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347").unwrap()); // = Keccak256(RLP([])) as of EIP-3675
}

/// Header part of a block on the chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockHeader {
    pub parent_hash: H256,
    pub ommers_hash: H256, // ommer = uncle
    pub coinbase: Address,
    pub state_root: H256,
    pub transactions_root: H256,
    pub receipt_root: H256,
    pub logs_bloom: Bloom,
    pub difficulty: U256,
    pub number: BlockNumber,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: Bytes,
    pub prev_randao: H256,
    pub nonce: u64,
    pub base_fee_per_gas: u64,
    pub withdrawals_root: H256,
    pub blob_gas_used: u64,
    pub excess_blob_gas: u64,
    pub parent_beacon_block_root: H256,
}

impl RLPEncode for BlockHeader {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.parent_hash)
            .encode_field(&self.ommers_hash)
            .encode_field(&self.coinbase)
            .encode_field(&self.state_root)
            .encode_field(&self.transactions_root)
            .encode_field(&self.receipt_root)
            .encode_field(&self.logs_bloom)
            .encode_field(&self.difficulty)
            .encode_field(&self.number)
            .encode_field(&self.gas_limit)
            .encode_field(&self.gas_used)
            .encode_field(&self.timestamp)
            .encode_field(&self.extra_data)
            .encode_field(&self.prev_randao)
            .encode_field(&self.nonce)
            .encode_field(&self.base_fee_per_gas)
            .encode_field(&self.withdrawals_root)
            .encode_field(&self.blob_gas_used)
            .encode_field(&self.excess_blob_gas)
            .encode_field(&self.parent_beacon_block_root)
            .finish();
    }
}

// The body of a block on the chain
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockBody {
    pub transactions: Vec<Transaction>,
    // TODO: ommers list is always empty, so we can remove it
    pub ommers: Vec<BlockHeader>,
    pub withdrawals: Vec<Withdrawal>,
}

impl BlockBody {
    pub const fn empty() -> Self {
        Self {
            transactions: Vec::new(),
            ommers: Vec::new(),
            withdrawals: Vec::new(),
        }
    }

    pub fn compute_transactions_root(&self) -> H256 {
        let transactions_iter: Vec<_> = self
            .transactions
            .iter()
            .enumerate()
            .map(|(i, tx)| {
                // Key: RLP(tx_index)
                let mut k = Vec::new();
                i.encode(&mut k);

                // Value: tx_type || RLP(tx)  if tx_type != 0
                //                   RLP(tx)  else
                let mut v = Vec::new();
                tx.encode_with_type(&mut v);

                (k, v)
            })
            .collect();
        let root = PatriciaMerkleTree::<_, _, Keccak256>::compute_hash_from_sorted_iter(
            &transactions_iter,
        );
        H256(root.into())
    }
}

pub fn compute_receipts_root(receipts: &[Receipt]) -> H256 {
    let receipts_iter: Vec<_> = receipts
        .iter()
        .enumerate()
        .map(|(i, receipt)| {
            // Key: RLP(index)
            let mut k = Vec::new();
            i.encode(&mut k);

            // Value: tx_type || RLP(receipt)  if tx_type != 0
            //                   RLP(receipt)  else
            let mut v = Vec::new();
            receipt.encode_with_type(&mut v);

            (k, v)
        })
        .collect();
    let root = PatriciaMerkleTree::<_, _, Keccak256>::compute_hash_from_sorted_iter(&receipts_iter);
    H256(root.into())
}

// See [EIP-4895](https://eips.ethereum.org/EIPS/eip-4895)
pub fn compute_withdrawals_root(withdrawals: &[Withdrawal]) -> H256 {
    let withdrawals_iter: Vec<_> = withdrawals
        .iter()
        .enumerate()
        .map(|(idx, withdrawal)| {
            let mut key = Vec::new();
            idx.encode(&mut key);
            let mut val = Vec::new();
            withdrawal.encode(&mut val);

            (key, val)
        })
        .collect();
    let root =
        PatriciaMerkleTree::<_, _, Keccak256>::compute_hash_from_sorted_iter(&withdrawals_iter);
    H256(root.into())
}

impl RLPEncode for BlockBody {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.transactions)
            .encode_field(&self.ommers)
            .encode_field(&self.withdrawals)
            .finish();
    }
}

impl BlockHeader {
    pub fn compute_block_hash(&self) -> H256 {
        let mut buf = vec![];
        self.encode(&mut buf);
        keccak(buf)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Withdrawal {
    #[serde(deserialize_with = "crate::serde_utils::u64::deser_hex_str")]
    index: u64,
    #[serde(deserialize_with = "crate::serde_utils::u64::deser_hex_str")]
    validator_index: u64,
    address: Address,
    amount: U256,
}

impl RLPEncode for Withdrawal {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.index)
            .encode_field(&self.validator_index)
            .encode_field(&self.address)
            .encode_field(&self.amount)
            .finish();
    }
}

#[cfg(test)]
mod test {

    use ethereum_types::H160;
    use hex_literal::hex;

    use super::*;

    #[test]
    fn test_compute_withdrawals_root() {
        // Source: https://github.com/ethereum/tests/blob/9760400e667eba241265016b02644ef62ab55de2/BlockchainTests/EIPTests/bc4895-withdrawals/amountIs0.json
        // "withdrawals" : [
        //             {
        //                 "address" : "0xc94f5374fce5edbc8e2a8697c15331677e6ebf0b",
        //                 "amount" : "0x00",
        //                 "index" : "0x00",
        //                 "validatorIndex" : "0x00"
        //             }
        //         ]
        // "withdrawalsRoot" : "0x48a703da164234812273ea083e4ec3d09d028300cd325b46a6a75402e5a7ab95"
        let withdrawals = vec![Withdrawal {
            index: 0x00,
            validator_index: 0x00,
            address: H160::from_slice(&hex!("c94f5374fce5edbc8e2a8697c15331677e6ebf0b")),
            amount: 0x00.into(),
        }];
        let expected_root = H256::from_slice(&hex!(
            "48a703da164234812273ea083e4ec3d09d028300cd325b46a6a75402e5a7ab95"
        ));
        let root = compute_withdrawals_root(&withdrawals);
        assert_eq!(root, expected_root);
    }
}
