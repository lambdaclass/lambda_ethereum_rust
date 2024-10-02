#[cfg(test)]
mod smoke_test {
    use std::{fs::File, io::BufReader};

    use ethereum_rust_core::{
        types::{Block, BlockHeader},
        H160, H256,
    };
    use ethereum_rust_storage::{EngineType, Store};

    use crate::{
        add_block,
        payload::{build_payload, BuildPayloadArgs},
    };

    #[test]
    fn test_add_block() {
        // Goal: Start from genesis, create new block, check balances in the new state.
        let store = test_store();
        let genesis_header = store.get_block_header(0).unwrap().unwrap();
        let block_1a = new_block(&store, &genesis_header);

        // Add first block. We'll make it canonical.
        add_block(&block_1a, &store).unwrap();

        store
            .set_canonical_block(1, block_1a.header.compute_block_hash())
            .unwrap();

        let retrieved_1 = store.get_block_header(1).unwrap().unwrap();

        assert_eq!(retrieved_1, block_1a.header);

        // Add second block at height 1. Will not be canonical.
        let block_1b = new_block(&store, &genesis_header);
        add_block(&block_1b, &store).expect("Could not add block 1b.");
        let retrieved_2 = store
            .get_block_header_by_hash(block_1b.header.compute_block_hash())
            .unwrap()
            .unwrap();

        assert_ne!(retrieved_1, retrieved_2);

        // Add a third block at height 2, from the non canonical block.

        let block_2 = new_block(&store, &block_1b.header);
        add_block(&block_1b, &store).expect("Could not add block 2.");
        let retrieved_3 = store
            .get_block_header_by_hash(block_2.header.compute_block_hash())
            .unwrap();

        assert!(!retrieved_3.is_none());
        assert!(store.get_canonical_block_hash(2).unwrap().is_none());
    }

    fn new_block(store: &Store, parent: &BlockHeader) -> Block {
        let args = BuildPayloadArgs {
            parent: parent.compute_block_hash(),
            timestamp: parent.timestamp + 12,
            fee_recipient: H160::random(),
            random: H256::random(),
            withdrawals: Vec::new(),
            beacon_root: Some(H256::random()),
            version: 1,
        };

        build_payload(&args, &store).unwrap()
    }

    fn test_store() -> Store {
        // Get genesis
        let file = File::open("../../test_data/genesis-execution-api.json")
            .expect("Failed to open genesis file");
        let reader = BufReader::new(file);
        let genesis = serde_json::from_reader(reader).expect("Failed to deserialize genesis file");

        // Build store with genesis
        let store =
            Store::new("store.db", EngineType::InMemory).expect("Failed to build DB for testing");

        store
            .add_initial_state(genesis)
            .expect("Failed to add genesis state");

        store
    }
}
