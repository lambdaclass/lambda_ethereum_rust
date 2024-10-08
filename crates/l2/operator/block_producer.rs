use std::{
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use ethereum_rust_rpc::types::fork_choice::{ForkChoiceState, PayloadAttributesV3};
use ethereum_types::H256;
use tokio::time::sleep;

use super::consensus_mock::ConsensusMock;

pub async fn start_block_producer() {
    // This is the genesis block hash
    let mut current_block_hash =
        H256::from_str("0x676fb5bffb4b4962edb2e1e03ac733597d5ba6ac290b230a2bf01448decae584")
            .unwrap();

    loop {
        let secret = std::fs::read("../../../jwt.hex").unwrap();
        let consensus_mock_client = ConsensusMock::new("http://localhost:8551", secret.into());

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

        let fork_choice_response = consensus_mock_client
            .engine_forkchoice_updated_v3(fork_choice_state, payload_attributes)
            .await
            .unwrap();

        let payload_id = fork_choice_response.payload_id.unwrap();

        let execution_payload_response = consensus_mock_client
            .engine_get_payload_v3(payload_id)
            .await
            .unwrap();

        let payload_status = consensus_mock_client
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
