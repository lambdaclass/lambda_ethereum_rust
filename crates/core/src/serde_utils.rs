use serde::{de::Error, Deserialize, Deserializer, Serializer};

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

pub mod u64 {
    use super::*;

    pub mod hex_str {
        use super::*;

        pub fn deserialize<'de, D>(d: D) -> Result<u64, D::Error>
        where
            D: Deserializer<'de>,
        {
            let value = String::deserialize(d)?;
            u64::from_str_radix(value.trim_start_matches("0x"), 16)
                .map_err(|_| D::Error::custom("Failed to deserialize u64 value"))
        }

        pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&format!("{:x}", value))
        }
    }

    pub fn deser_dec_str<'de, D>(d: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(d)?;
        value
            .parse()
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
