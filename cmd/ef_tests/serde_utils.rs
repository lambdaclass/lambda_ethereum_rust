pub mod h160 {
    use std::str::FromStr;

    use ethereum_rust_core::H160;
    use serde::{de::Error, Deserialize, Deserializer};
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
