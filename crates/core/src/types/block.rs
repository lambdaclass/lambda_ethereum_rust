use crate::{
    rlp::{
        constants::RLP_NULL, decode::RLPDecode, encode::RLPEncode, error::RLPDecodeError,
        structs::Encoder,
    },
    types::Receipt,
    Address, H256, U256,
};
use bytes::Bytes;
use patricia_merkle_tree::PatriciaMerkleTree;
use sha3::Keccak256;

pub type BlockNumber = u64;
pub type Bloom = [u8; 256];

/// Header part of a block on the chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockHeader {
    parent_hash: H256,
    ommers_hash: H256,
    coinbase: Address,
    state_root: H256,
    transactions_root: H256,
    receipt_root: H256,
    logs_bloom: Bloom,
    difficulty: U256,
    number: BlockNumber,
    gas_limit: u64,
    gas_used: u64,
    timestamp: u64,
    extra_data: Bytes,
    prev_randao: H256,
    nonce: u64,
    base_fee_per_gas: u64,
    withdrawals_root: H256,
    blob_gas_used: u64,
    excess_blob_gas: u64,
    parent_beacon_block_root: H256,
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
    transactions: Vec<Transaction>,
    // TODO: ommers list is always empty, so we can remove it
    ommers: Vec<BlockHeader>,
    withdrawals: Vec<Withdrawal>,
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

    pub fn compute_receipts_root(&self, receipts: Vec<Receipt>) -> H256 {
        let receipts_iter: Vec<_> = receipts
            .iter()
            .enumerate()
            .map(|(i, receipt)| {
                // Key: RLP(index)
                let mut k = Vec::new();
                i.encode(&mut k);

                // Value: RLP(receipt)
                let mut v = Vec::new();
                receipt.encode(&mut v);

                (k, v)
            })
            .collect();
        let root =
            PatriciaMerkleTree::<_, _, Keccak256>::compute_hash_from_sorted_iter(&receipts_iter);
        H256(root.into())
    }
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Withdrawal {
    index: u64,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Transaction {
    LegacyTransaction(LegacyTransaction),
    EIP1559Transaction(EIP1559Transaction),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TxType {
    Legacy = 0x00,
    EIP2930 = 0x01,
    EIP1559 = 0x02,
    EIP4844 = 0x03,
}

impl Transaction {
    pub fn encode_with_type(&self, buf: &mut dyn bytes::BufMut) {
        // tx_type || RLP(tx)  if tx_type != 0
        //            RLP(tx)  else
        match self {
            // Legacy transactions don't have a prefix
            Transaction::LegacyTransaction(_) => {}
            _ => buf.put_u8(self.tx_type() as u8),
        }

        self.encode(buf);
    }

    pub fn tx_type(&self) -> TxType {
        match self {
            Transaction::LegacyTransaction(_) => TxType::Legacy,
            Transaction::EIP1559Transaction(_) => TxType::EIP1559,
        }
    }
}

impl RLPEncode for Transaction {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        match self {
            Transaction::LegacyTransaction(t) => t.encode(buf),
            Transaction::EIP1559Transaction(t) => t.encode(buf),
        };
    }
}

/// The transaction's kind: call or create.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TxKind {
    Call(Address),
    Create,
}

impl RLPEncode for TxKind {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        match self {
            Self::Call(address) => address.encode(buf),
            Self::Create => buf.put_u8(RLP_NULL),
        }
    }
}

impl RLPDecode for TxKind {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let first_byte = rlp.first().ok_or(RLPDecodeError::InvalidLength)?;
        if *first_byte == RLP_NULL {
            return Ok((Self::Create, &rlp[1..]));
        }
        Address::decode_unfinished(rlp).map(|(t, rest)| (Self::Call(t), rest))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyTransaction {
    nonce: u64,
    gas_price: U256,
    gas: u64,
    /// The recipient of the transaction.
    /// Create transactions contain a [`null`](RLP_NULL) value in this field.
    to: TxKind,
    value: U256,
    data: Bytes,
    v: U256,
    r: U256,
    s: U256,
}

impl RLPEncode for LegacyTransaction {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.nonce)
            .encode_field(&self.gas_price)
            .encode_field(&self.gas)
            .encode_field(&self.to)
            .encode_field(&self.value)
            .encode_field(&self.data)
            .encode_field(&self.v)
            .encode_field(&self.r)
            .encode_field(&self.s)
            .finish();
    }
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

impl RLPEncode for EIP1559Transaction {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.chain_id)
            .encode_field(&self.signer_nonce)
            .encode_field(&self.max_priority_fee_per_gas)
            .encode_field(&self.max_fee_per_gas)
            .encode_field(&self.gas_limit)
            .encode_field(&self.destination)
            .encode_field(&self.amount)
            .encode_field(&self.payload)
            .encode_field(&self.access_list)
            .encode_field(&self.signature_y_parity)
            .encode_field(&self.signature_r)
            .encode_field(&self.signature_s)
            .finish();
    }
}

#[cfg(test)]
mod tests {
    use super::{BlockBody, LegacyTransaction};
    use crate::{
        types::{Receipt, Transaction, TxKind},
        U256,
    };
    use hex_literal::hex;

    #[test]
    fn test_compute_transactions_root() {
        let mut body = BlockBody::empty();
        let tx = LegacyTransaction {
            nonce: 0,
            gas_price: 0x0a.into(),
            gas: 0x05f5e100,
            to: TxKind::Call(hex!("1000000000000000000000000000000000000000").into()),
            value: 0.into(),
            data: Default::default(),
            v: U256::from(0x1b),
            r: U256::from(hex!(
                "7e09e26678ed4fac08a249ebe8ed680bf9051a5e14ad223e4b2b9d26e0208f37"
            )),
            s: U256::from(hex!(
                "5f6e3f188e3e6eab7d7d3b6568f5eac7d687b08d307d3154ccd8c87b4630509b"
            )),
        };
        body.transactions.push(Transaction::LegacyTransaction(tx));
        let expected_root =
            hex!("8151d548273f6683169524b66ca9fe338b9ce42bc3540046c828fd939ae23bcb");
        let result = body.compute_transactions_root();

        assert_eq!(result, expected_root.into());
    }

    #[test]
    fn test_compute_receipts_root() {
        // example taken from
        // https://github.com/ethereum/go-ethereum/blob/f8aa62353666a6368fb3f1a378bd0a82d1542052/cmd/evm/testdata/1/exp.json#L18
        let body = BlockBody::empty();
        let succeeded = true;
        let cumulative_gas_used = 0x5208;
        let bloom = [0x00; 256];
        let logs = vec![];
        let receipt = Receipt::new(succeeded, cumulative_gas_used, bloom, logs);

        let result = body.compute_receipts_root(vec![receipt]);
        let expected_root =
            hex!("056b23fbba480696b65fe5a59b8f2148a1299103c4f57df839233af2cf4ca2d2");
        assert_eq!(result, expected_root.into());
    }
}
