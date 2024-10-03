#[cfg(test)]
mod test {
    use std::{fs::File, io::BufReader};

    use ethereum_rust_core::{
        types::{Block, BlockHeader},
        H160, H256,
    };
    use ethereum_rust_storage::{EngineType, Store};

    use crate::{
        add_block, is_canonical, new_head,
        payload::{build_payload, BuildPayloadArgs},
    };

    #[test]
    fn test_add_block() {
        // Goal: Start from genesis, create new block, check balances in the new state.
        let store = test_store();
        let genesis_header = store.get_block_header(0).unwrap().unwrap();
        let genesis_hash = genesis_header.compute_block_hash();

        // Add first block. We'll make it canonical.
        let block_1a = new_block(&store, &genesis_header);
        let hash_1a = block_1a.header.compute_block_hash();
        add_block(&block_1a, &store).unwrap();
        store.set_canonical_block(1, hash_1a).unwrap();
        let retrieved_1a = store.get_block_header(1).unwrap().unwrap();

        assert_eq!(retrieved_1a, block_1a.header);
        assert!(is_canonical(&store, 1, hash_1a).unwrap());

        // Add second block at height 1. Will not be canonical.
        let block_1b = new_block(&store, &genesis_header);
        let hash_1b = block_1b.header.compute_block_hash();
        add_block(&block_1b, &store).expect("Could not add block 1b.");
        let retrieved_1b = store.get_block_header_by_hash(hash_1b).unwrap().unwrap();

        assert_ne!(retrieved_1a, retrieved_1b);
        assert!(!is_canonical(&store, 1, hash_1b).unwrap());

        // Add a third block at height 2, child to the non canonical block.
        let block_2 = new_block(&store, &block_1b.header);
        let hash_2 = block_2.header.compute_block_hash();
        add_block(&block_2, &store).expect("Could not add block 2.");
        let retrieved_2 = store.get_block_header_by_hash(hash_2).unwrap();

        assert!(retrieved_2.is_some());
        assert!(store.get_canonical_block_hash(2).unwrap().is_none());

        // Receive block 2 as new head.
        new_head(
            &store,
            block_2.header.compute_block_hash(),
            genesis_header.compute_block_hash(),
            genesis_header.compute_block_hash(),
        )
        .unwrap();

        // Check that canonical blocks changed to the new branch.
        assert!(is_canonical(&store, 0, genesis_hash).unwrap());
        assert!(is_canonical(&store, 1, hash_1b).unwrap());
        assert!(is_canonical(&store, 2, hash_2).unwrap());
        assert!(!is_canonical(&store, 1, hash_1a).unwrap());
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

        build_payload(&args, store).unwrap()
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
