use ethereum_rust_storage::Store;
pub mod l1_watcher;
pub mod proof_data_provider;
pub mod xxx;

pub async fn start_operator(store: Store) {
    let l1_watcher = tokio::spawn(l1_watcher::start_l1_watcher(store.clone()));
    let current_block_hash = {
        let current_block_number = store.get_latest_block_number().unwrap().unwrap();
        store
            .get_canonical_block_hash(current_block_number)
            .unwrap()
            .unwrap()
    };
    let xxx = tokio::spawn(xxx::start_xxx(current_block_hash, store));
    let proof_data_provider = tokio::spawn(proof_data_provider::start_proof_data_provider());

    tokio::try_join!(l1_watcher, xxx, proof_data_provider).unwrap();
}
