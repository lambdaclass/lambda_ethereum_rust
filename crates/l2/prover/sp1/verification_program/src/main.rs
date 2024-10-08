#![no_main]
#![allow(unused_imports)]
#![allow(dead_code)]

use ethereum_rust_blockchain::{validate_gas_used, verify_blob_gas_usage};
use ethereum_rust_core::types::{
    validate_block_header, validate_cancun_header_fields, Receipt, Transaction,
};

sp1_zkvm::entrypoint!(main);

pub fn main() {
    let head_block_bytes = sp1_zkvm::io::read::<Vec<u8>>();
    let parent_block_header_bytes = sp1_zkvm::io::read::<Vec<u8>>();

    // Make Inputs public.
    sp1_zkvm::io::commit(&head_block_bytes);
    sp1_zkvm::io::commit(&parent_block_header_bytes);

    let block = <ethereum_rust_core::types::Block as ethereum_rust_rlp::decode::RLPDecode>::decode(
        &head_block_bytes,
    )
    .unwrap();
    let parent_block_header =
        <ethereum_rust_core::types::BlockHeader as ethereum_rust_rlp::decode::RLPDecode>::decode(
            &parent_block_header_bytes,
        )
        .unwrap();

    let block_header_is_valid = validate_block(&block, &parent_block_header);

    sp1_zkvm::io::commit(&block_header_is_valid);
}

fn validate_block(
    block: &ethereum_rust_core::types::Block,
    parent_block_header: &ethereum_rust_core::types::BlockHeader,
) -> bool {
    validate_block_header(&block.header, parent_block_header).unwrap();
    validate_cancun_header_fields(&block.header, parent_block_header).unwrap();
    verify_blob_gas_usage(block).is_ok()
}
