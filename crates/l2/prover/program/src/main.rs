#![no_main]

sp1_zkvm::entrypoint!(main);

/// Mock of zkVM program
pub fn main() {
    let head_block_bytes = sp1_zkvm::io::read::<Vec<u8>>();
    let parent_header_bytes = sp1_zkvm::io::read::<Vec<u8>>();
    // let memory_db = sp1_zkvm::io::read::<MemoryDB>();

    // setup data from inputs
    let block = <ethereum_rust_core::types::Block as ethereum_rust_rlp::decode::RLPDecode>::decode(
        &head_block_bytes,
    )
    .unwrap();

    let parent_header =
        <ethereum_rust_core::types::BlockHeader as ethereum_rust_rlp::decode::RLPDecode>::decode(
            &parent_header_bytes,
        )
        .unwrap();

    // make inputs public.
    sp1_zkvm::io::commit(&block);
    sp1_zkvm::io::commit(&parent_header);
    // sp1_zkvm::io::commit(&memory_db);

    // setup CacheDB in order to use execute_block()
    // let mut cache_db = CacheDB::new(memory_db);
    // println!("executing block");

    // let block_receipts = execute_block(&block, &mut cache_db).unwrap();

    // sp1_zkvm::io::commit(&block_receipts);
}
