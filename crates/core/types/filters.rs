use ethereum_rust_rlp::{
    decode::{decode_rlp_item, RLPDecode},
    encode::RLPEncode,
    error::RLPDecodeError,
    structs::{Decoder, Encoder},
};
use ethereum_types::{Address, H160, H256};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
/// Address filter used to filter Logs.
pub enum AddressFilter {
    Single(H160),
    Many(Vec<H160>),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
/// Topic filter used to filter Logs.
pub enum TopicFilter {
    Topic(H256),
    Topics(Vec<H256>),
}

#[derive(Debug, Clone)]
pub struct LogsFilter {
    /// The oldest block from which to start
    /// retrieving logs.
    /// Will default to `latest` if not provided.
    pub from_block: u64,
    /// Up to which block to stop retrieving logs.
    /// Will default to `latest` if not provided.
    pub to_block: u64,
    /// The addresses from where the logs origin from.
    pub addresses: Vec<Address>,
    /// Which topics to filter.
    pub topics: Vec<TopicFilter>,
}

impl RLPEncode for LogsFilter {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        Encoder::new(buf)
            .encode_field(&self.from_block)
            .encode_field(&self.to_block)
            .encode_field(&self.topics)
            .encode_field(&self.addresses)
            .finish();
    }
}

impl RLPDecode for LogsFilter {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        let (from_block, decoder) = decoder.decode_field("from_block")?;
        let (to_block, decoder) = decoder.decode_field("to_block")?;
        let (topics, decoder) = decoder.decode_field("topics")?;
        let (addresses, decoder) = decoder.decode_field("addresses")?;
        Ok((
            LogsFilter {
                from_block,
                to_block,
                topics,
                addresses,
            },
            decoder.finish()?,
        ))
    }
}
impl RLPDecode for TopicFilter {
    fn decode_unfinished(rlp: &[u8]) -> Result<(Self, &[u8]), RLPDecodeError> {
        let decoder = Decoder::new(rlp)?;
        // Since a topic can be a topic or a list of topics,
        // let's first check if it is a list, and if not,
        // try to decode the single variant
        match decode_rlp_item(rlp)? {
            (false, encoded_topics, remainder) => {
                let (decoded_topics, _) = RLPDecode::decode_unfinished(encoded_topics)?;
                Ok((TopicFilter::Topics(decoded_topics), remainder))
            }
            (true, encoded_topic, remainder) => {
                let (decoded_topics, _) = RLPDecode::decode_unfinished(encoded_topic)?;
                Ok((TopicFilter::Topics(decoded_topics), remainder))
            }
        }
    }
}

impl RLPEncode for TopicFilter {
    fn encode(&self, buf: &mut dyn bytes::BufMut) {
        // Since a topic can be a topic or a list of topics,
        // let's first check if it is a list, and if not,
        // try to decode the single variant
        match self {
            TopicFilter::Topic(topic) => {
                RLPEncode::encode(topic, buf);
            }
            TopicFilter::Topics(topics) => {
                RLPEncode::encode(topics, buf);
            }
        }
    }
}
