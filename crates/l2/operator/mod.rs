use ethereum_rust_storage::Store;

pub mod block_producer;
pub mod l1_tx_sender;
pub mod l1_watcher;
pub mod proof_data_provider;

pub async fn start_operator(store: Store) {
    let l1_tx_sender = tokio::spawn(l1_tx_sender::start_l1_tx_sender());
    let l1_watcher = tokio::spawn(l1_watcher::start_l1_watcher(store.clone()));
    let current_block_hash = {
        let current_block_number = store.get_latest_block_number().unwrap().unwrap();
        store
            .get_canonical_block_hash(current_block_number)
            .unwrap()
            .unwrap()
    };
    let block_producer = tokio::spawn(block_producer::start_block_producer(current_block_hash));
    let proof_data_provider = tokio::spawn(proof_data_provider::start_proof_data_provider());

    tokio::try_join!(
        l1_tx_sender,
        l1_watcher,
        block_producer,
        proof_data_provider
    )
    .unwrap();
}
