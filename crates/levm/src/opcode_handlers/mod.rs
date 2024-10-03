pub mod arithmetic;
pub mod bitwise_comparison;
pub mod keccak;
pub mod environment;
pub mod block;
pub mod stack_memory_storage_flow;
pub mod push;
pub mod dup;
pub mod exchange;
pub mod logging;
pub mod system;

use crate::{call_frame::CallFrame, opcodes::Opcode, vm::VM, vm_result::VMError};
use ethereum_types::{Address, U256, U512};


use std::str::FromStr;
pub fn address_to_word(address: Address) -> U256 {
    // This unwrap can't panic, as Address are 20 bytes long and U256 use 32 bytes
    U256::from_str(&format!("{address:?}")).unwrap()
}
