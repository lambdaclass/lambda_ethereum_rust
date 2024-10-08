use crate::operator::engine::Engine;
use ethereum_rust_rpc::types::fork_choice::{ForkChoiceState, PayloadAttributesV3};
use ethereum_types::H256;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

pub async fn start_block_producer(current_block_hash: H256) {
    let mut current_block_hash = current_block_hash;
    loop {
        let secret = std::fs::read("../../../jwt.hex").unwrap();
        let engine = Engine::new("http://localhost:8551", secret.into());

        let fork_choice_state = ForkChoiceState {
            head_block_hash: current_block_hash,
            safe_block_hash: current_block_hash,
            finalized_block_hash: current_block_hash,
        };
        let payload_attributes = PayloadAttributesV3 {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ..Default::default()
        };

        let fork_choice_response = engine
            .engine_forkchoice_updated_v3(fork_choice_state, payload_attributes)
            .await
            .unwrap();

        let payload_id = fork_choice_response.payload_id.unwrap();

        let execution_payload_response = engine.engine_get_payload_v3(payload_id).await.unwrap();

        let payload_status = engine
            .engine_new_payload_v3(
                execution_payload_response.execution_payload,
                Default::default(),
                Default::default(),
            )
            .await
            .unwrap();

        current_block_hash = payload_status.latest_valid_hash.unwrap();

        sleep(Duration::from_secs(5)).await;
    }
}
