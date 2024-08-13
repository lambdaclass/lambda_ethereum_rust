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
        U256::from_dec_str(&value)
            .map_err(|e| D::Error::custom(e.to_string()))
            .map(Some)
    }

    pub fn deser_dec_str<'de, D>(d: D) -> Result<U256, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(d)?;
        U256::from_dec_str(&value).map_err(|e| D::Error::custom(e.to_string()))
    }

    pub fn deser_hex_str<'de, D>(d: D) -> Result<U256, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(d)?;
        U256::from_str_radix(value.trim_start_matches("0x"), 16)
            .map_err(|_| D::Error::custom("Failed to deserialize u64 to u256 value"))
    }

    pub fn deser_hex_or_dec_str<'de, D>(d: D) -> Result<U256, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(d)?;
        if value.starts_with("0x") {
            U256::from_str_radix(value.trim_start_matches("0x"), 16)
                .map_err(|_| D::Error::custom("Failed to deserialize u64 to u256 value"))
        } else {
            U256::from_dec_str(&value).map_err(|e| D::Error::custom(e.to_string()))
        }
    }
}

pub mod h160 {
    use super::*;

    use ethereum_types::H160;
    use std::str::FromStr;

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

    pub mod hex_str {
        use super::*;

        pub fn deserialize<'de, D>(d: D) -> Result<u64, D::Error>
        where
            D: Deserializer<'de>,
        {
            let value = String::deserialize(d)?;
            let res = u64::from_str_radix(value.trim_start_matches("0x"), 16)
                .map_err(|_| D::Error::custom("Failed to deserialize u64 value"));
            res
        }

        pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&format!("{:#x}", value))
        }
    }

    pub mod hex_str_opt {
        use serde::Serialize;

        use super::*;

        pub fn serialize<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            Option::<String>::serialize(&value.map(|v| format!("{:#x}", v)), serializer)
        }

        pub fn deserialize<'de, D>(d: D) -> Result<Option<u64>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let value = String::deserialize(d)?;
            u64::from_str_radix(value.trim_start_matches("0x"), 16)
                .map_err(|_| D::Error::custom("Failed to deserialize u64 value"))
                .map(Some)
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

    pub fn deser_hex_or_dec_str<'de, D>(d: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(d)?;
        if value.starts_with("0x") {
            u64::from_str_radix(value.trim_start_matches("0x"), 16)
                .map_err(|_| D::Error::custom("Failed to deserialize u64 value"))
        } else {
            value
                .parse()
                .map_err(|_| D::Error::custom("Failed to deserialize u64 value"))
        }
    }
}

/// Serializes to and deserializes from 0x prefixed hex string
pub mod bytes {
    use ::bytes::Bytes;

    use super::*;

    pub fn deserialize<'de, D>(d: D) -> Result<Bytes, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(d)?;
        let bytes = hex::decode(value.trim_start_matches("0x"))
            .map_err(|e| D::Error::custom(e.to_string()))?;
        Ok(Bytes::from(bytes))
    }

    pub fn serialize<S>(value: &Bytes, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("0x{:x}", value))
    }

    pub mod vec {
        use super::*;

        pub fn deserialize<'de, D>(d: D) -> Result<Vec<Bytes>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let value = Vec::<String>::deserialize(d)?;
            let mut output = Vec::new();
            for str in value {
                let bytes = hex::decode(str.trim_start_matches("0x"))
                    .map_err(|e| D::Error::custom(e.to_string()))?
                    .into();
                output.push(bytes);
            }
            Ok(output)
        }
    }
}

/// Serializes to and deserializes from 0x prefixed hex string
pub mod bool {
    use super::*;

    pub fn deserialize<'de, D>(d: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(d)?;
        Ok(u8::from_str_radix(value.trim_start_matches("0x"), 16)
            .map_err(|_| D::Error::custom("Failed to deserialize hex string to boolean value"))?
            != 0)
    }

    pub fn serialize<S>(value: &bool, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:#x}", *value as u8))
    }
}
