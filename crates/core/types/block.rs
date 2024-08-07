use super::{
    ReceiptBlockInfo, BASE_FEE_MAX_CHANGE_DENOMINATOR, ELASTICITY_MULTIPLIER,
    GAS_LIMIT_ADJUSTMENT_FACTOR, GAS_LIMIT_MINIMUM,
};
use crate::{
    rlp::{
        decode::RLPDecode,
        encode::RLPEncode,
        structs::{Decoder, Encoder},
    },
    types::Receipt,
    Address, H256, U256,
};
use bytes::Bytes;
use ethereum_types::Bloom;
use keccak_hash::keccak;
use patricia_merkle_tree::PatriciaMerkleTree;
use serde::{Deserialize, Serialize};
use sha3::Keccak256;

use std::cmp::{max, Ordering};

use super::Transaction;

pub use serializable::BlockSerializable;

pub type BlockNumber = u64;
pub type BlockHash = H256;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref DEFAULT_OMMERS_HASH: H256 = H256::from_slice(&hex::decode("1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347").unwrap()); // = Keccak256(RLP([])) as of EIP-3675
}
#[derive(PartialEq, Eq, Debug)]
pub struct Block {
    pub header: BlockHeader,
    pub body: BlockBody,
}

impl RLPEncode for Block {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.header)
            .encode_field(&self.body.transactions)
            .encode_field(&self.body.ommers)
            .encode_optional_field(&self.body.withdrawals)
            .finish();
    }
}

impl RLPDecode for Block {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), crate::rlp::error::RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (header, decoder) = decoder.decode_field("header")?;
        let (transactions, decoder) = decoder.decode_field("transactions")?;
        let (ommers, decoder) = decoder.decode_field("ommers")?;
        let (withdrawals, decoder) = decoder.decode_optional_field();
        let remaining = decoder.finish()?;
        let body = BlockBody {
            transactions,
            ommers,
            withdrawals,
        };
        let block = Block { header, body };
        Ok((block, remaining))
    }
}

/// Header part of a block on the chain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockHeader {
    pub parent_hash: H256,
    #[serde(rename(serialize = "sha3Uncles"))]
    pub ommers_hash: H256, // ommer = uncle
    #[serde(rename(serialize = "miner"))]
    pub coinbase: Address,
    pub state_root: H256,
    pub transactions_root: H256,
    pub receipt_root: H256,
    pub logs_bloom: Bloom,
    pub difficulty: U256,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub number: BlockNumber,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub gas_limit: u64,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub gas_used: u64,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub timestamp: u64,
    #[serde(with = "crate::serde_utils::bytes")]
    pub extra_data: Bytes,
    #[serde(rename(serialize = "mixHash"))]
    pub prev_randao: H256,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub nonce: u64,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub base_fee_per_gas: u64,
    pub withdrawals_root: Option<H256>,
    #[serde(with = "crate::serde_utils::u64::hex_str_opt")]
    pub blob_gas_used: Option<u64>,
    #[serde(with = "crate::serde_utils::u64::hex_str_opt")]
    pub excess_blob_gas: Option<u64>,
    pub parent_beacon_block_root: Option<H256>,
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
            .encode_field(&self.nonce.to_be_bytes())
            .encode_field(&self.base_fee_per_gas)
            .encode_optional_field(&self.withdrawals_root)
            .encode_optional_field(&self.blob_gas_used)
            .encode_optional_field(&self.excess_blob_gas)
            .encode_optional_field(&self.parent_beacon_block_root)
            .finish();
    }
}

impl RLPDecode for BlockHeader {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), crate::rlp::error::RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (parent_hash, decoder) = decoder.decode_field("parent_hash")?;
        let (ommers_hash, decoder) = decoder.decode_field("ommers_hash")?;
        let (coinbase, decoder) = decoder.decode_field("coinbase")?;
        let (state_root, decoder) = decoder.decode_field("state_root")?;
        let (transactions_root, decoder) = decoder.decode_field("transactions_root")?;
        let (receipt_root, decoder) = decoder.decode_field("receipt_root")?;
        let (logs_bloom, decoder) = decoder.decode_field("logs_bloom")?;
        let (difficulty, decoder) = decoder.decode_field("difficulty")?;
        let (number, decoder) = decoder.decode_field("number")?;
        let (gas_limit, decoder) = decoder.decode_field("gas_limit")?;
        let (gas_used, decoder) = decoder.decode_field("gas_used")?;
        let (timestamp, decoder) = decoder.decode_field("timestamp")?;
        let (extra_data, decoder) = decoder.decode_field("extra_data")?;
        let (prev_randao, decoder) = decoder.decode_field("prev_randao")?;
        let (nonce, decoder) = decoder.decode_field("nonce")?;
        let nonce = u64::from_be_bytes(nonce);
        let (base_fee_per_gas, decoder) = decoder.decode_field("base_fee_per_gas")?;
        let (withdrawals_root, decoder) = decoder.decode_optional_field();
        let (blob_gas_used, decoder) = decoder.decode_optional_field();
        let (excess_blob_gas, decoder) = decoder.decode_optional_field();
        let (parent_beacon_block_root, decoder) = decoder.decode_optional_field();

        Ok((
            BlockHeader {
                parent_hash,
                ommers_hash,
                coinbase,
                state_root,
                transactions_root,
                receipt_root,
                logs_bloom,
                difficulty,
                number,
                gas_limit,
                gas_used,
                timestamp,
                extra_data,
                prev_randao,
                nonce,
                base_fee_per_gas,
                withdrawals_root,
                blob_gas_used,
                excess_blob_gas,
                parent_beacon_block_root,
            },
            decoder.finish()?,
        ))
    }
}

// The body of a block on the chain
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BlockBody {
    pub transactions: Vec<Transaction>,
    // TODO: ommers list is always empty, so we can remove it
    #[serde(rename(serialize = "uncles"))]
    pub ommers: Vec<BlockHeader>,
    pub withdrawals: Option<Vec<Withdrawal>>,
}

impl BlockBody {
    pub const fn empty() -> Self {
        Self {
            transactions: Vec::new(),
            ommers: Vec::new(),
            withdrawals: Some(Vec::new()),
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
                tx.encode(&mut v);

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
            receipt.encode(&mut v);

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
            .encode_optional_field(&self.withdrawals)
            .finish();
    }
}

impl RLPDecode for BlockBody {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), crate::rlp::error::RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (transactions, decoder) = decoder.decode_field("transactions")?;
        let (ommers, decoder) = decoder.decode_field("ommers")?;
        let (withdrawals, decoder) = decoder.decode_optional_field();
        Ok((
            BlockBody {
                transactions,
                ommers,
                withdrawals,
            },
            decoder.finish()?,
        ))
    }
}

impl BlockHeader {
    pub fn compute_block_hash(&self) -> H256 {
        let mut buf = vec![];
        self.encode(&mut buf);
        keccak(buf)
    }

    pub fn receipt_info(&self) -> ReceiptBlockInfo {
        ReceiptBlockInfo {
            block_hash: self.compute_block_hash(),
            block_number: self.number,
            gas_used: self.gas_used,
            blob_gas_used: self.blob_gas_used.unwrap_or_default(),
            root: self.state_root,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Withdrawal {
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub index: u64,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub validator_index: u64,
    pub address: Address,
    #[serde(with = "crate::serde_utils::u64::hex_str")]
    pub amount: u64,
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

impl RLPDecode for Withdrawal {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), crate::rlp::error::RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (index, decoder) = decoder.decode_field("index")?;
        let (validator_index, decoder) = decoder.decode_field("validator_index")?;
        let (address, decoder) = decoder.decode_field("address")?;
        let (amount, decoder) = decoder.decode_field("amount")?;
        Ok((
            Withdrawal {
                index,
                validator_index,
                address,
                amount,
            },
            decoder.finish()?,
        ))
    }
}

// Checks that the gas_limit fits the gas bounds set by its parent block
fn check_gas_limit(gas_limit: u64, parent_gas_limit: u64) -> bool {
    let max_adjustment_delta = parent_gas_limit / GAS_LIMIT_ADJUSTMENT_FACTOR;

    gas_limit < parent_gas_limit + max_adjustment_delta
        && gas_limit > parent_gas_limit - max_adjustment_delta
        && gas_limit >= GAS_LIMIT_MINIMUM
}

// Calculates the base fee for the current block based on its gas_limit and parent's gas and fee
// Returns None if the block gas limit is not valid in relation to its parent's gas limit
fn calculate_base_fee_per_gas(
    block_gas_limit: u64,
    parent_gas_limit: u64,
    parent_gas_used: u64,
    parent_base_fee_per_gas: u64,
) -> Option<u64> {
    // Check gas limit, if the check passes we can also rest assured that none of the
    // following divisions will have zero as a divider
    if !check_gas_limit(block_gas_limit, parent_gas_limit) {
        return None;
    }

    let parent_gas_target = parent_gas_limit / ELASTICITY_MULTIPLIER;

    Some(match parent_gas_used.cmp(&parent_gas_target) {
        Ordering::Equal => parent_base_fee_per_gas,
        Ordering::Greater => {
            let gas_used_delta = parent_gas_used - parent_gas_target;

            let parent_fee_gas_delta = parent_base_fee_per_gas * gas_used_delta;
            let target_fee_gas_delta = parent_fee_gas_delta / parent_gas_target;

            let base_fee_per_gas_delta =
                max(target_fee_gas_delta / BASE_FEE_MAX_CHANGE_DENOMINATOR, 1);

            parent_base_fee_per_gas + base_fee_per_gas_delta
        }
        Ordering::Less => {
            let gas_used_delta = parent_gas_target - parent_gas_used;

            let parent_fee_gas_delta = parent_base_fee_per_gas * gas_used_delta;
            let target_fee_gas_delta = parent_fee_gas_delta / parent_gas_target;

            let base_fee_per_gas_delta = target_fee_gas_delta / BASE_FEE_MAX_CHANGE_DENOMINATOR;

            parent_base_fee_per_gas - base_fee_per_gas_delta
        }
    })
}

pub fn validate_block_header(header: &BlockHeader, parent_header: &BlockHeader) -> bool {
    if header.gas_used > header.gas_limit {
        return false;
    }
    let expected_base_fee_per_gas = if let Some(base_fee) = calculate_base_fee_per_gas(
        header.gas_limit,
        parent_header.gas_limit,
        parent_header.gas_used,
        parent_header.base_fee_per_gas,
    ) {
        base_fee
    } else {
        return false;
    };

    expected_base_fee_per_gas == header.base_fee_per_gas
        && header.timestamp > parent_header.timestamp
        && header.number == parent_header.number + 1
        && header.extra_data.len() <= 32
        && header.difficulty.is_zero()
        && header.nonce == 0
        && header.ommers_hash == *DEFAULT_OMMERS_HASH
        && header.parent_hash == parent_header.compute_block_hash()
}

#[allow(unused)]
mod serializable {
    use super::*;

    #[derive(Debug, Serialize)]
    pub struct BlockSerializable {
        hash: H256,
        #[serde(flatten)]
        header: BlockHeader,
        #[serde(flatten)]
        body: BlockBodyWrapper,
    }

    #[derive(Debug, Serialize)]
    #[serde(untagged)]
    enum BlockBodyWrapper {
        Full(BlockBody),
        OnlyHashes(OnlyHashesBlockBody),
    }

    #[derive(Debug, Serialize)]
    struct OnlyHashesBlockBody {
        // Only tx hashes
        pub transactions: Vec<H256>,
        pub uncles: Vec<BlockHeader>,
        pub withdrawals: Vec<Withdrawal>,
    }

    impl BlockSerializable {
        pub fn from_block(
            header: BlockHeader,
            body: BlockBody,
            full_transactions: bool,
        ) -> BlockSerializable {
            let body = if full_transactions {
                BlockBodyWrapper::Full(body)
            } else {
                BlockBodyWrapper::OnlyHashes(OnlyHashesBlockBody {
                    transactions: body.transactions.iter().map(|t| t.compute_hash()).collect(),
                    uncles: body.ommers,
                    withdrawals: body.withdrawals.unwrap(),
                })
            };
            let hash = header.compute_block_hash();
            BlockSerializable { hash, header, body }
        }
    }
}

#[cfg(test)]
mod test {

    use std::str::FromStr;

    use ethereum_types::H160;
    use hex_literal::hex;
    use serializable::BlockSerializable;

    use crate::types::{EIP1559Transaction, TxKind};

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
            amount: 0x00_u64,
        }];
        let expected_root = H256::from_slice(&hex!(
            "48a703da164234812273ea083e4ec3d09d028300cd325b46a6a75402e5a7ab95"
        ));
        let root = compute_withdrawals_root(&withdrawals);
        assert_eq!(root, expected_root);
    }

    #[test]
    fn test_validate_block_header() {
        let parent_block = BlockHeader {
            parent_hash: H256::from_str(
                "0x0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            ommers_hash: H256::from_str(
                "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
            )
            .unwrap(),
            coinbase: Address::zero(),
            state_root: H256::from_str(
                "0x590245a249decc317041b8dc7141cec0559c533efb82221e4e0a30a6456acf8b",
            )
            .unwrap(),
            transactions_root: H256::from_str(
                "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
            )
            .unwrap(),
            receipt_root: H256::from_str(
                "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
            )
            .unwrap(),
            logs_bloom: Bloom::from([0; 256]),
            difficulty: U256::zero(),
            number: 0,
            gas_limit: 0x016345785d8a0000,
            gas_used: 0,
            timestamp: 0,
            extra_data: Bytes::new(),
            prev_randao: H256::zero(),
            nonce: 0x0000000000000000,
            base_fee_per_gas: 0x07,
            withdrawals_root: Some(
                H256::from_str(
                    "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
                )
                .unwrap(),
            ),
            blob_gas_used: Some(0x00),
            excess_blob_gas: Some(0x00),
            parent_beacon_block_root: Some(H256::zero()),
        };
        let block = BlockHeader {
            parent_hash: H256::from_str(
                "0x1ac1bf1eef97dc6b03daba5af3b89881b7ae4bc1600dc434f450a9ec34d44999",
            )
            .unwrap(),
            ommers_hash: H256::from_str(
                "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
            )
            .unwrap(),
            coinbase: Address::from_str("0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba").unwrap(),
            state_root: H256::from_str(
                "0x9de6f95cb4ff4ef22a73705d6ba38c4b927c7bca9887ef5d24a734bb863218d9",
            )
            .unwrap(),
            transactions_root: H256::from_str(
                "0x578602b2b7e3a3291c3eefca3a08bc13c0d194f9845a39b6f3bcf843d9fed79d",
            )
            .unwrap(),
            receipt_root: H256::from_str(
                "0x035d56bac3f47246c5eed0e6642ca40dc262f9144b582f058bc23ded72aa72fa",
            )
            .unwrap(),
            logs_bloom: Bloom::from([0; 256]),
            difficulty: U256::zero(),
            number: 1,
            gas_limit: 0x016345785d8a0000,
            gas_used: 0xa8de,
            timestamp: 0x03e8,
            extra_data: Bytes::new(),
            prev_randao: H256::zero(),
            nonce: 0x0000000000000000,
            base_fee_per_gas: 0x07,
            withdrawals_root: Some(
                H256::from_str(
                    "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
                )
                .unwrap(),
            ),
            blob_gas_used: Some(0x00),
            excess_blob_gas: Some(0x00),
            parent_beacon_block_root: Some(H256::zero()),
        };
        assert!(validate_block_header(&block, &parent_block))
    }

    #[test]
    fn serialize_block() {
        let block_header = BlockHeader {
            parent_hash: H256::from_str(
                "0x1ac1bf1eef97dc6b03daba5af3b89881b7ae4bc1600dc434f450a9ec34d44999",
            )
            .unwrap(),
            ommers_hash: H256::from_str(
                "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
            )
            .unwrap(),
            coinbase: Address::from_str("0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba").unwrap(),
            state_root: H256::from_str(
                "0x9de6f95cb4ff4ef22a73705d6ba38c4b927c7bca9887ef5d24a734bb863218d9",
            )
            .unwrap(),
            transactions_root: H256::from_str(
                "0x578602b2b7e3a3291c3eefca3a08bc13c0d194f9845a39b6f3bcf843d9fed79d",
            )
            .unwrap(),
            receipt_root: H256::from_str(
                "0x035d56bac3f47246c5eed0e6642ca40dc262f9144b582f058bc23ded72aa72fa",
            )
            .unwrap(),
            logs_bloom: Bloom::from([0; 256]),
            difficulty: U256::zero(),
            number: 1,
            gas_limit: 0x016345785d8a0000,
            gas_used: 0xa8de,
            timestamp: 0x03e8,
            extra_data: Bytes::new(),
            prev_randao: H256::zero(),
            nonce: 0x0000000000000000,
            base_fee_per_gas: 0x07,
            withdrawals_root: Some(
                H256::from_str(
                    "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
                )
                .unwrap(),
            ),
            blob_gas_used: Some(0x00),
            excess_blob_gas: Some(0x00),
            parent_beacon_block_root: Some(H256::zero()),
        };

        let tx = EIP1559Transaction {
            nonce: 0,
            max_fee_per_gas: 78,
            max_priority_fee_per_gas: 17,
            to: TxKind::Call(Address::from_slice(
                &hex::decode("6177843db3138ae69679A54b95cf345ED759450d").unwrap(),
            )),
            value: 3000000000000000_u64.into(),
            data: Bytes::from_static(b"0x1568"),
            signature_r: U256::from_str_radix(
                "151ccc02146b9b11adf516e6787b59acae3e76544fdcd75e77e67c6b598ce65d",
                16,
            )
            .unwrap(),
            signature_s: U256::from_str_radix(
                "64c5dd5aae2fbb535830ebbdad0234975cd7ece3562013b63ea18cc0df6c97d4",
                16,
            )
            .unwrap(),
            signature_y_parity: false,
            chain_id: 3151908,
            gas_limit: 63000,
            access_list: vec![(
                Address::from_slice(
                    &hex::decode("6177843db3138ae69679A54b95cf345ED759450d").unwrap(),
                ),
                vec![],
            )],
        };

        let block_body = BlockBody {
            transactions: vec![Transaction::EIP1559Transaction(tx)],
            ommers: vec![],
            withdrawals: Some(vec![]),
        };

        let block = BlockSerializable::from_block(block_header, block_body, true);
        let expected_block = r#"{"hash":"0x63d6a2504601fc2db0ccf02a28055eb0cdb40c444ecbceec0f613980421a035e","parentHash":"0x1ac1bf1eef97dc6b03daba5af3b89881b7ae4bc1600dc434f450a9ec34d44999","sha3Uncles":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","miner":"0x2adc25665018aa1fe0e6bc666dac8fc2697ff9ba","stateRoot":"0x9de6f95cb4ff4ef22a73705d6ba38c4b927c7bca9887ef5d24a734bb863218d9","transactionsRoot":"0x578602b2b7e3a3291c3eefca3a08bc13c0d194f9845a39b6f3bcf843d9fed79d","receiptRoot":"0x035d56bac3f47246c5eed0e6642ca40dc262f9144b582f058bc23ded72aa72fa","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","difficulty":"0x0","number":"0x1","gasLimit":"0x16345785d8a0000","gasUsed":"0xa8de","timestamp":"0x3e8","extraData":"0x","mixHash":"0x0000000000000000000000000000000000000000000000000000000000000000","nonce":"0x0","baseFeePerGas":"0x7","withdrawalsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","blobGasUsed":"0x0","excessBlobGas":"0x0","parentBeaconBlockRoot":"0x0000000000000000000000000000000000000000000000000000000000000000","transactions":[{"type":"0x2","nonce":"0x0","to":"0x6177843db3138ae69679a54b95cf345ed759450d","gas":"0xf618","value":"0xaa87bee538000","input":"0x307831353638","maxPriorityFeePerGas":"0x11","maxFeePerGas":"0x4e","gasPrice":"0x4e","accessList":[{"address":"0x6177843db3138ae69679a54b95cf345ed759450d","storageKeys":[]}],"chainId":"0x301824","yParity":"0x0","r":"0x151ccc02146b9b11adf516e6787b59acae3e76544fdcd75e77e67c6b598ce65d","s":"0x64c5dd5aae2fbb535830ebbdad0234975cd7ece3562013b63ea18cc0df6c97d4"}],"uncles":[],"withdrawals":[]}"#;
        assert_eq!(serde_json::to_string(&block).unwrap(), expected_block)
    }
}
