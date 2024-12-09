use bytes::Bytes;
use ethereum_types::{Address, Bloom, BloomInput, H256};
use ethrex_rlp::{
    decode::{get_rlp_bytes_item_payload, is_encoded_as_bytes, RLPDecode},
    encode::RLPEncode,
    error::RLPDecodeError,
    structs::{Decoder, Encoder},
};
use serde::{Deserialize, Serialize};

use crate::types::TxType;
pub type Index = u64;

/// Result of a transaction
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Receipt {
    pub tx_type: TxType,
    pub succeeded: bool,
    pub cumulative_gas_used: u64,
    pub bloom: Bloom,
    pub logs: Vec<Log>,
}

impl Receipt {
    pub fn new(tx_type: TxType, succeeded: bool, cumulative_gas_used: u64, logs: Vec<Log>) -> Self {
        Self {
            tx_type,
            succeeded,
            cumulative_gas_used,
            bloom: bloom_from_logs(&logs),
            logs,
        }
    }
    pub fn inner_encode_receipt(&self) -> Vec<u8> {
        let mut encode_buff = match self.tx_type {
            TxType::Legacy => {
                vec![]
            }
            _ => {
                vec![self.tx_type as u8]
            }
        };
        Encoder::new(&mut encode_buff)
            .encode_field(&self.succeeded)
            .encode_field(&self.cumulative_gas_used)
            .encode_field(&self.bloom)
            .encode_field(&self.logs)
            .finish();
        encode_buff
    }
}

fn bloom_from_logs(logs: &[Log]) -> Bloom {
    let mut bloom = Bloom::zero();
    for log in logs {
        bloom.accrue(BloomInput::Raw(log.address.as_ref()));
        for topic in log.topics.iter() {
            bloom.accrue(BloomInput::Raw(topic.as_ref()));
        }
    }
    bloom
}

impl RLPEncode for Receipt {
    /// Receipts can be encoded in the following formats:
    /// A) Legacy receipts: rlp(LegacyTransaction)
    /// B) Non legacy receipts: rlp(Bytes(tx_type | rlp(receipt))).
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        // tx_type || RLP(receipt)  if tx_type != 0
        //            RLP(receipt)  else
        match self.tx_type {
            TxType::Legacy => {
                let legacy_encoded = self.inner_encode_receipt();
                buf.put_slice(&legacy_encoded);
            }
            _ => {
                let typed_recepipt_encoded = self.inner_encode_receipt();
                let bytes = Bytes::from(typed_recepipt_encoded);
                bytes.encode(buf);
            }
        };
    }
}

impl RLPDecode for Receipt {
    /// Receipts can be encoded in the following formats:
    /// A) Legacy receipts: rlp(LegacyTransaction)
    /// B) Non legacy receipts: rlp(Bytes(tx_type | rlp(receipt))).
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        if is_encoded_as_bytes(rlp)? {
            let payload = get_rlp_bytes_item_payload(rlp)?;
            let tx_type = payload.first().ok_or(RLPDecodeError::InvalidLength)?;
            let receipt_encoding = &payload[1..];
            let tx_type = match tx_type {
                0x0 => TxType::Legacy,
                0x1 => TxType::EIP2930,
                0x2 => TxType::EIP1559,
                0x3 => TxType::EIP4844,
                // 0x7e => TxType::PrivilegedL2Transaction,
                ty => {
                    return Err(RLPDecodeError::Custom(format!(
                        "Invalid transaction type: {ty}"
                    )))
                }
            };
            // FIXME: Remove unwrap
            let decoder = Decoder::new(receipt_encoding).unwrap();
            let (succeeded, decoder) = decoder.decode_field("succeded").unwrap();
            let (cumulative_gas_used, decoder) =
                decoder.decode_field("cumulative gas used").unwrap();
            let (bloom, decoder) = decoder.decode_field("bloom").unwrap();
            let (logs, decoder) = decoder.decode_field("logs").unwrap();
            Ok((
                Receipt {
                    tx_type,
                    succeeded,
                    bloom,
                    logs,
                    cumulative_gas_used,
                },
                decoder.finish().unwrap(),
            ))
        } else {
            let decoder = Decoder::new(rlp).unwrap();
            let (succeeded, decoder) = decoder.decode_field("succeded").unwrap();
            let (cumulative_gas_used, decoder) =
                decoder.decode_field("cumulative gas used").unwrap();
            let (bloom, decoder) = decoder.decode_field("bloom").unwrap();
            let (logs, decoder) = decoder.decode_field("logs").unwrap();
            Ok((
                Receipt {
                    tx_type: TxType::Legacy,
                    succeeded,
                    bloom,
                    logs,
                    cumulative_gas_used,
                },
                decoder.finish().unwrap(),
            ))
        }
    }
}

/// Data record produced during the execution of a transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Log {
    pub address: Address,
    pub topics: Vec<H256>,
    pub data: Bytes,
}

impl RLPEncode for Log {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.address)
            .encode_field(&self.topics)
            .encode_field(&self.data)
            .finish();
    }
}

impl RLPDecode for Log {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (address, decoder) = decoder.decode_field("address")?;
        let (topics, decoder) = decoder.decode_field("topics")?;
        let (data, decoder) = decoder.decode_field("data")?;
        let log = Log {
            address,
            topics,
            data,
        };
        Ok((log, decoder.finish()?))
    }
}
