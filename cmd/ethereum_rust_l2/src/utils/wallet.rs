use bytes::Bytes;
use ethereum_rust_core::types::{EIP1559Transaction, GenericTransaction, TxKind};
use ethereum_rust_l2::utils::eth_client::{errors::EthClientError, EthClient};
use ethereum_rust_rlp::encode::RLPEncode;
use ethereum_types::{Address, H256, U256};
use keccak_hash::keccak;
use libsecp256k1::SecretKey;

#[derive(Default)]
pub struct Overrides {
    pub from: Option<Address>,
    pub value: Option<U256>,
    pub nonce: Option<u64>,
    pub chain_id: Option<u64>,
    pub gas_limit: Option<u64>,
    pub gas_price: Option<u64>,
    pub priority_gas_price: Option<u64>,
}

pub async fn deploy(
    deployer: Address,
    deployer_private_key: SecretKey,
    bytecode: Bytes,
    overrides: Overrides,
    client: EthClient,
) -> Result<(H256, Address), eyre::Error> {
    let mut tx =
        make_eip1559_transaction(&client, TxKind::Create, deployer, bytecode, overrides).await?;

    let hash = client
        .send_eip1559_transaction(&mut tx, deployer_private_key)
        .await?;

    let encoded_from = deployer.encode_to_vec();
    let encoded_nonce = tx.nonce.encode_to_vec();
    let mut encoded = vec![(0xc0 + encoded_from.len() + encoded_nonce.len()) as u8];
    encoded.extend(encoded_from.clone());
    encoded.extend(encoded_nonce.clone());
    let deployed_address = Address::from(keccak(encoded));

    Ok((hash, deployed_address))
}

pub async fn call(
    to: Address,
    calldata: Bytes,
    overrides: Overrides,
    client: &EthClient,
) -> Result<String, eyre::Error> {
    let tx = GenericTransaction {
        to: TxKind::Call(to),
        input: calldata,
        value: overrides.value.unwrap_or_default(),
        from: overrides.from.unwrap_or_default(),
        gas: overrides.gas_limit,
        gas_price: overrides
            .gas_price
            .unwrap_or(client.get_gas_price().await?.as_u64()),
        ..Default::default()
    };

    Ok(client.call(tx).await?)
}

pub async fn send(
    from: Address,
    to: Address,
    calldata: Bytes,
    sender_private_key: SecretKey,
    overrides: Overrides,
    client: &EthClient,
) -> Result<H256, eyre::Error> {
    let mut tx =
        make_eip1559_transaction(client, TxKind::Call(to), from, calldata, overrides).await?;

    Ok(client
        .send_eip1559_transaction(&mut tx, sender_private_key)
        .await?)
}

#[allow(clippy::too_many_arguments)]
async fn make_eip1559_transaction(
    client: &EthClient,
    to: TxKind,
    from: Address,
    data: Bytes,
    overrides: Overrides,
) -> Result<EIP1559Transaction, EthClientError> {
    let mut tx = EIP1559Transaction {
        to,
        data,
        value: overrides.value.unwrap_or_default(),
        chain_id: overrides
            .chain_id
            .unwrap_or(client.get_chain_id().await?.as_u64()),
        nonce: overrides.nonce.unwrap_or(client.get_nonce(from).await?),
        max_fee_per_gas: overrides
            .gas_price
            .unwrap_or(client.get_gas_price().await?.as_u64()),
        max_priority_fee_per_gas: overrides.priority_gas_price.unwrap_or_default(),
        ..Default::default()
    };
    tx.gas_limit = overrides
        .gas_limit
        .unwrap_or(client.estimate_gas(tx.clone()).await?);

    Ok(tx)
}
