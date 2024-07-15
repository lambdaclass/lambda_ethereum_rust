use serde::{de::Error, Deserialize, Deserializer};

pub mod u256 {
    use super::*;
    use ethereum_types::U256;
    use serde_json::Number;

    pub fn deser_number<'de, D>(d: D) -> Result<U256, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Number::deserialize(d)?.to_string();
        U256::from_dec_str(&value).map_err(|e| D::Error::custom(e.to_string()))
    }

    pub fn deser_number_opt<'de, D>(d: D) -> Result<Option<U256>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Number::deserialize(d)?.to_string();
        Ok(Some(
            U256::from_dec_str(&value).map_err(|e| D::Error::custom(e.to_string()))?,
        ))
    }

    pub fn deser_dec_str<'de, D>(d: D) -> Result<U256, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(d)?;
        U256::from_dec_str(&value).map_err(|e| D::Error::custom(e.to_string()))
    }
}

pub mod h160 {
    use std::str::FromStr;

    use super::*;
    use ethereum_types::H160;
    pub fn deser_hex_str<'de, D>(d: D) -> Result<H160, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(d)?;
        if value.is_empty() {
            Ok(H160::zero())
        } else {
            H160::from_str(value.trim_start_matches("0x"))
                .map_err(|_| D::Error::custom("Failed to deserialize H160 value"))
        }
    }
}

pub mod u64 {
    use super::*;

    pub fn deser_dec_str<'de, D>(d: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(d)?;
        value
            .parse()
            .map_err(|_| D::Error::custom("Failed to deserialize u64 value"))
    }

    pub fn deser_hex_str<'de, D>(d: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(d)?;
        u64::from_str_radix(value.trim_start_matches("0x"), 16)
            .map_err(|_| D::Error::custom("Failed to deserialize u64 value"))
    }
}

pub mod bytes {
    use ::bytes::Bytes;

    use super::*;

    pub fn deser_hex_str<'de, D>(d: D) -> Result<Bytes, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(d)?;
        let bytes = hex::decode(value.trim_start_matches("0x"))
            .map_err(|e| D::Error::custom(e.to_string()))?;
        Ok(Bytes::from(bytes))
    }
}
