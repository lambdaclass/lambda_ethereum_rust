use serde::{de::Error, Deserialize, Deserializer, Serializer};
pub mod h160 {
    use ethereum_rust_core::H160;
    use serde::{de::Error, Deserialize, Deserializer};
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
            u64::from_str_radix(value.trim_start_matches("0x"), 16)
                .map_err(|_| D::Error::custom("Failed to deserialize u64 value"))
        }

        pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&format!("{:#x}", value))
        }
    }
}
