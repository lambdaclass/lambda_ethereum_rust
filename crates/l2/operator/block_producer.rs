use bytes::Bytes;
use std::{
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::error;

use ethereum_rust_rpc::types::fork_choice::{ForkChoiceState, PayloadAttributesV3};
use ethereum_types::H256;
use tokio::time::sleep;

use super::consensus_mock::ConsensusMock;

pub async fn start_block_producer() {
    // This is the genesis block hash
    let mut current_block_hash =
        H256::from_str("0x493dba6364c3e0b575b81efe8b255eb76d8fcd302517557beac9ead7816ea7a3")
            .unwrap();

    loop {
        let secret = Bytes::from_static(include_bytes!(
            "/Users/manu/Documents/Lambda/eth/ethereum_rust/jwt.hex"
        ));
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

        let fork_choice_response = match consensus_mock_client
            .engine_forkchoice_updated_v3(fork_choice_state, payload_attributes)
            .await
        {
            Ok(response) => response,
            Err(e) => {
                error!("Error sending forkChoice: {e}");
                continue;
            }
        };

        let payload_id = fork_choice_response.payload_id.unwrap();

        let execution_payload_response = match consensus_mock_client
            .engine_get_payload_v3(payload_id)
            .await
        {
            Ok(response) => response,
            Err(e) => {
                error!("Error sending getPayload: {e}");
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
                continue;
            }
        };

        current_block_hash = payload_status.latest_valid_hash.unwrap();

        sleep(Duration::from_secs(9)).await;
    }
}
