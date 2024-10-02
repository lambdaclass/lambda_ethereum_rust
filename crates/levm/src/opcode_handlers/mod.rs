use std::str::FromStr;

use ethereum_types::{Address, U256};

pub mod arithmetic;
pub mod bitwise_comparison;
pub mod crypto;
pub mod environment;
pub mod block;
pub mod stack_memory_flow;
pub mod push;
pub mod dup;
pub mod swap;
pub mod logging;
pub mod system;

/// Shifts the value to the right by 255 bits and checks the most significant bit is a 1
pub fn is_negative(value: U256) -> bool {
    value.bit(255)
}
/// negates a number in two's complement
pub fn negate(value: U256) -> U256 {
    !value + U256::one()
}

pub fn address_to_word(address: Address) -> U256 {
    // This unwrap can't panic, as Address are 20 bytes long and U256 use 32 bytes
    U256::from_str(&format!("{address:?}")).unwrap()
}
