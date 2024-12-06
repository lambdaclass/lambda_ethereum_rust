use bytes::Bytes;
use ethereum_types::{Address, Bloom, BloomInput, H256};
use ethrex_rlp::{
    decode::{get_rlp_bytes_item_payload, RLPDecode},
    encode::RLPEncode,
    error::RLPDecodeError,
    structs::{Decoder, Encoder},
};
use serde::{Deserialize, Serialize};

use super::TxType;
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
                Encoder::new(buf)
                    .encode_field(&self.succeeded)
                    .encode_field(&self.cumulative_gas_used)
                    .encode_field(&self.bloom)
                    .encode_field(&self.logs)
                    .finish();
            }
            _ => {
                let mut tmp_buff = vec![self.tx_type as u8];
                Encoder::new(&mut tmp_buff)
                    .encode_field(&self.succeeded)
                    .encode_field(&self.cumulative_gas_used)
                    .encode_field(&self.bloom)
                    .encode_field(&self.logs)
                    .finish();
                let bytes = Bytes::from(tmp_buff);
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
        let payload = get_rlp_bytes_item_payload(rlp);
        let tx_type = match payload.first() {
            Some(tx_type) if *tx_type < 0x7f => match tx_type {
                0x0 => TxType::Legacy,
                0x1 => TxType::EIP2930,
                0x2 => TxType::EIP1559,
                0x3 => TxType::EIP4844,
                0x7e => TxType::Privileged,
                ty => {
                    return Err(RLPDecodeError::Custom(format!(
                        "Invalid transaction type: {ty}"
                    )))
                }
            },
            Some(_) => TxType::Legacy,
            None => return Err(RLPDecodeError::InvalidLength),
        };
        let Some(receipt_encoding) = &payload.get(1..) else {
            return Err(RLPDecodeError::InvalidLength);
        };
        let decoder = Decoder::new(receipt_encoding)?;
        let (succeeded, decoder) = decoder.decode_field("succeeded")?;
        let (cumulative_gas_used, decoder) = decoder.decode_field("cumulative_gas_used")?;
        let (bloom, decoder) = decoder.decode_field("bloom")?;
        let (logs, decoder) = decoder.decode_field("logs")?;
        let receipt = Receipt {
            tx_type,
            succeeded,
            cumulative_gas_used,
            bloom,
            logs,
        };
        Ok((receipt, decoder.finish()?))
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
