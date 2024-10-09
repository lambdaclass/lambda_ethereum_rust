use bytes::Bytes;
use std::{
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::{debug, error};

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
        // let secret = Bytes::from_static(include_bytes!(
        //     "/Users/federicoborello/Desktop/ethereum_lambda/ethereum_rust/l2/sp1-execute-block/crates/l2/jwt.hex"
        // ));
        let secret = Bytes::from_static(&[0]);
        let consensus_mock_client = ConsensusMock::new("http://localhost:8551", secret);

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

        let response = consensus_mock_client
            .engine_forkchoice_updated_v3(fork_choice_state, payload_attributes)
            .await;
        debug!("ForkChoice Response: {response:?}");
        let fork_choice_response = match response {
            Ok(r) => r,
            Err(e) => {
                error!("Error sending forkChoice: {e}");
                sleep(Duration::from_secs(10)).await;
                continue;
            }
        };

        let payload_id = fork_choice_response.payload_id.unwrap();

        let payload_response = consensus_mock_client
            .engine_get_payload_v3(payload_id)
            .await;
        let execution_payload_response = match payload_response {
            Ok(response) => response,
            Err(e) => {
                error!("Error sending getPayload: {e}");
                sleep(Duration::from_secs(10)).await;
                continue;
            }
        };

        let payload_status = match consensus_mock_client
            .engine_new_payload_v3(
                execution_payload_response.execution_payload,
                Default::default(),
                Default::default(),
            )
            .await
        {
            Ok(response) => response,
            Err(e) => {
                error!("Error sending newPayload: {e}");
                sleep(Duration::from_secs(10)).await;
                continue;
            }
        };

        current_block_hash = payload_status.latest_valid_hash.unwrap();

        sleep(Duration::from_secs(9)).await;
    }
}
