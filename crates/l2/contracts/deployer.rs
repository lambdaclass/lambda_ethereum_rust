use bytes::Bytes;
use ethereum_rust_l2::utils::eth_client::{eth_sender::Overrides, EthClient};
use ethereum_types::{Address, H160, H256};
use keccak_hash::keccak;
use libsecp256k1::SecretKey;

// 0x4e59b44847b379578588920cA78FbF26c0B4956C
const DETERMINISTIC_CREATE2_ADDRESS: Address = H160([
    0x4e, 0x59, 0xb4, 0x48, 0x47, 0xb3, 0x79, 0x57, 0x85, 0x88, 0x92, 0x0c, 0xa7, 0x8f, 0xbf, 0x26,
    0xc0, 0xb4, 0x95, 0x6c,
]);

#[tokio::main]
async fn main() {
    let eth_client = EthClient::new("http://localhost:8545");

    // let (on_chain_proposer_deployment_tx_hash, on_chain_proposer_address) = eth_client
    //     .deploy(deployer, deployer_private_key, init_code, overrides)
    //     .await
    //     .unwrap();

    // let (bridge_deployment_tx_hash, bridge_address) = eth_client
    //     .deploy(deployer, deployer_private_key, init_code, overrides)
    //     .await
    //     .unwrap();
}

async fn deploy_on_chain_proposer(
    deployer: Address,
    deployer_private_key: SecretKey,
    init_code: Bytes,
    overrides: Overrides,
    eth_client: &EthClient,
) -> (H256, Address) {
    eth_client
        .deploy(deployer, deployer_private_key, init_code, overrides)
        .await
        .unwrap()
}

async fn deploy_bridge(eth_client: &EthClient) {}

fn create2_address(deployer: Address, salt: [u8; 32], init_code: Bytes) -> Address {
    Address::from_slice(
        keccak(
            [
                &[0xff],
                deployer.as_bytes(),
                &salt,
                keccak(init_code).as_bytes(),
            ]
            .concat(),
        )
        .as_bytes(),
    )
}
