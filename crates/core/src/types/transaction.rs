use bytes::Bytes;
use ethereum_types::{Address, H256, U256};
use secp256k1::{ecdsa::RecoveryId, Message, SECP256K1};
use sha3::{Digest, Keccak256};

use crate::rlp::{
    constants::RLP_NULL, decode::RLPDecode, encode::RLPEncode, error::RLPDecodeError,
    structs::Encoder,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Transaction {
    LegacyTransaction(LegacyTransaction),
    EIP1559Transaction(EIP1559Transaction),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacyTransaction {
    pub nonce: u64,
    pub gas_price: u64,
    pub gas: u64,
    /// The recipient of the transaction.
    /// Create transactions contain a [`null`](RLP_NULL) value in this field.
    pub to: TxKind,
    pub value: U256,
    pub data: Bytes,
    pub v: U256,
    pub r: U256,
    pub s: U256,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EIP1559Transaction {
    pub chain_id: u64,
    pub signer_nonce: u64,
    pub max_priority_fee_per_gas: u64,
    pub max_fee_per_gas: u64,
    pub gas_limit: u64,
    pub destination: Address,
    pub amount: U256,
    pub payload: Bytes,
    pub access_list: Vec<(Address, Vec<H256>)>,
    pub signature_y_parity: bool,
    pub signature_r: U256,
    pub signature_s: U256,
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

impl Transaction {
    pub fn sender(&self) -> Address {
        match self {
            Transaction::LegacyTransaction(tx) => {
                let signature_y_parity = match self.chain_id() {
                    Some(chain_id) => tx.v.as_u64().saturating_sub(35 + chain_id * 2) != 0,
                    None => tx.v.as_u64().saturating_sub(27) != 0,
                };
                let mut buf = vec![];
                Encoder::new(&mut buf)
                    .encode_field(&tx.nonce)
                    .encode_field(&tx.gas_price)
                    .encode_field(&tx.gas)
                    .encode_field(&tx.to)
                    .encode_field(&tx.value)
                    .encode_field(&tx.data)
                    .finish();
                recover_address(&tx.r, &tx.s, signature_y_parity, &Bytes::from(buf))
            }
            Transaction::EIP1559Transaction(tx) => {
                let mut buf = vec![self.tx_type() as u8];
                Encoder::new(&mut buf)
                    .encode_field(&tx.chain_id)
                    .encode_field(&tx.signer_nonce)
                    .encode_field(&tx.max_priority_fee_per_gas)
                    .encode_field(&tx.max_fee_per_gas)
                    .encode_field(&tx.gas_limit)
                    .encode_field(&tx.destination)
                    .encode_field(&tx.amount)
                    .encode_field(&tx.payload)
                    .encode_field(&tx.access_list)
                    .finish();
                recover_address(
                    &tx.signature_r,
                    &tx.signature_s,
                    tx.signature_y_parity,
                    &Bytes::from(buf),
                )
            }
        }
    }

    pub fn gas_limit(&self) -> u64 {
        match self {
            Transaction::LegacyTransaction(tx) => tx.gas,
            Transaction::EIP1559Transaction(tx) => tx.gas_limit,
        }
    }

    pub fn gas_price(&self) -> u64 {
        match self {
            Transaction::LegacyTransaction(tx) => tx.gas_price,
            Transaction::EIP1559Transaction(tx) => tx.max_fee_per_gas,
        }
    }

    pub fn to(&self) -> TxKind {
        match self {
            Transaction::LegacyTransaction(tx) => tx.to.clone(),
            Transaction::EIP1559Transaction(tx) => TxKind::Call(tx.destination),
        }
    }

    pub fn value(&self) -> U256 {
        match self {
            Transaction::LegacyTransaction(tx) => tx.value,
            Transaction::EIP1559Transaction(tx) => tx.amount,
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

    pub fn nonce(&self) -> u64 {
        match self {
            Transaction::LegacyTransaction(tx) => tx.nonce,
            Transaction::EIP1559Transaction(tx) => tx.signer_nonce,
        }
    }

    pub fn data(&self) -> &Bytes {
        match self {
            Transaction::LegacyTransaction(tx) => &tx.data,
            Transaction::EIP1559Transaction(tx) => &tx.payload,
        }
    }
}

fn recover_address(
    signature_r: &U256,
    signature_s: &U256,
    signature_y_parity: bool,
    message: &Bytes,
) -> Address {
    // Create signature
    let mut signature_bytes = [0; 64];
    signature_r.to_big_endian(&mut signature_bytes[0..32]);
    signature_s.to_big_endian(&mut signature_bytes[32..]);
    let signature = secp256k1::ecdsa::RecoverableSignature::from_compact(
        &signature_bytes,
        RecoveryId::from_i32(signature_y_parity as i32).unwrap(), // cannot fail
    )
    .unwrap();
    // Hash message
    let msg_digest: [u8; 32] = Keccak256::new_with_prefix(message.as_ref())
        .finalize()
        .into();
    // Recover public key
    let public = SECP256K1
        .recover_ecdsa(&Message::from_digest(msg_digest), &signature)
        .unwrap();
    // Hash public key to obtain address
    let hash = Keccak256::new_with_prefix(&public.serialize_uncompressed()[1..]).finalize();
    Address::from_slice(&hash[12..])
}

#[cfg(test)]
mod tests {
    use hex_literal::hex;

    use crate::{
        types::{
            compute_receipts_root, BlockBody, LegacyTransaction, Receipt, Transaction, TxKind,
            TxType,
        },
        U256,
    };

    #[test]
    fn test_compute_transactions_root() {
        let mut body = BlockBody::empty();
        let tx = LegacyTransaction {
            nonce: 0,
            gas_price: 0x0a,
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
        let tx_type = TxType::Legacy;
        let succeeded = true;
        let cumulative_gas_used = 0x5208;
        let bloom = [0x00; 256];
        let logs = vec![];
        let receipt = Receipt::new(tx_type, succeeded, cumulative_gas_used, bloom, logs);

        let result = compute_receipts_root(&[receipt]);
        let expected_root =
            hex!("056b23fbba480696b65fe5a59b8f2148a1299103c4f57df839233af2cf4ca2d2");
        assert_eq!(result, expected_root.into());
    }
}
