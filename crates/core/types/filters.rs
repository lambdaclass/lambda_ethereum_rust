use ethereum_types::{H160, H256};
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
    Topic(Option<H256>),
    Topics(Vec<Option<H256>>),
}
