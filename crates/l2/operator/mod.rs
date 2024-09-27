pub mod block_producer;
pub mod l1_tx_sender;
pub mod l1_watcher;

pub async fn start_operator() {
    let l1_tx_sender = tokio::spawn(l1_tx_sender::start_l1_tx_sender());
    let l1_watcher = tokio::spawn(l1_watcher::start_l1_watcher());
    let block_producer = tokio::spawn(block_producer::start_block_producer());

    tokio::try_join!(l1_tx_sender, l1_watcher, block_producer).unwrap();
}
