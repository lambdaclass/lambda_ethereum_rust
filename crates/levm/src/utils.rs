use ethereum_types::U256;

pub fn u256_to_i128(value: U256) -> i128 {
    if value.bit(255) {
        -((!value + U256::one()).as_u128() as i128)
    } else {
        value.as_u128() as i128
    }
}
