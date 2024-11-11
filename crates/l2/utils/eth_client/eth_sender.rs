use crate::utils::eth_client::{
    errors::{CallError, EthClientError},
    EthClient, RpcResponse,
};
use bytes::Bytes;
use ethereum_rust_core::types::{EIP1559Transaction, GenericTransaction, TxKind, TxType};
use ethereum_rust_rlp::encode::RLPEncode;
use ethereum_rust_rpc::utils::{RpcRequest, RpcRequestId};
use ethereum_types::{Address, U256};
use keccak_hash::{keccak, H256};
use libsecp256k1::SecretKey;
use serde_json::json;

#[derive(Default, Clone)]
pub struct Overrides {
    pub from: Option<Address>,
    pub value: Option<U256>,
    pub nonce: Option<u64>,
    pub chain_id: Option<u64>,
    pub gas_limit: Option<u64>,
    pub gas_price: Option<u64>,
    pub priority_gas_price: Option<u64>,
}

impl EthClient {
    pub async fn call(
        &self,
        to: Address,
        calldata: Bytes,
        overrides: Overrides,
    ) -> Result<String, EthClientError> {
        let tx = GenericTransaction {
            to: TxKind::Call(to),
            input: calldata,
            value: overrides.value.unwrap_or_default(),
            from: overrides.from.unwrap_or_default(),
            gas: overrides.gas_limit,
            gas_price: overrides
                .gas_price
                .unwrap_or(self.get_gas_price().await?.as_u64()),
            ..Default::default()
        };

        let request = RpcRequest {
            id: RpcRequestId::Number(1),
            jsonrpc: "2.0".to_string(),
            method: "eth_call".to_string(),
            params: Some(vec![
                json!({
                    "to": match tx.to {
                        TxKind::Call(addr) => format!("{addr:#x}"),
                        TxKind::Create => format!("{:#x}", Address::zero()),
                    },
                    "input": format!("{:#x}", tx.input),
                    "value": format!("{}", tx.value),
                    "from": format!("{:#x}", tx.from),
                }),
                json!("latest"),
            ]),
        };

        match self.send_request(request).await {
            Ok(RpcResponse::Success(result)) => serde_json::from_value(result.result)
                .map_err(CallError::SerdeJSONError)
                .map_err(EthClientError::from),
            Ok(RpcResponse::Error(error_response)) => {
                Err(CallError::RPCError(error_response.error.message).into())
            }
            Err(error) => Err(error),
        }
    }

    pub async fn send(
        &self,
        calldata: Bytes,
        from: Address,
        to: TxKind,
        sender_private_key: SecretKey,
        overrides: Overrides,
    ) -> Result<H256, EthClientError> {
        let mut tx = self
            .make_eip1559_transaction(to, from, calldata, overrides)
            .await?;

        self.send_eip1559_transaction(&mut tx, sender_private_key)
            .await
    }

    pub async fn deploy(
        &self,
        deployer: Address,
        deployer_private_key: SecretKey,
        init_code: Bytes,
        overrides: Overrides,
    ) -> Result<(H256, Address), EthClientError> {
        let deploy_tx_hash = self
            .send(
                init_code,
                deployer,
                TxKind::Create,
                deployer_private_key,
                overrides,
            )
            .await?;

        let encoded_from = deployer.encode_to_vec();
        // FIXME: We'll probably need to use nonce - 1 since it was updated above.
        let encoded_nonce = self.get_nonce(deployer).await.unwrap().encode_to_vec();
        let mut encoded = vec![(0xc0 + encoded_from.len() + encoded_nonce.len()) as u8];
        encoded.extend(encoded_from.clone());
        encoded.extend(encoded_nonce.clone());
        let deployed_address = Address::from(keccak(encoded));

        Ok((deploy_tx_hash, deployed_address))
    }

    async fn make_eip1559_transaction(
        &self,
        to: TxKind,
        from: Address,
        data: Bytes,
        overrides: Overrides,
    ) -> Result<EIP1559Transaction, EthClientError> {
        let generic_transaction = GenericTransaction {
            r#type: TxType::EIP1559,
            from,
            to: to.clone(),
            input: data.clone(),
            nonce: overrides.nonce.or(self.get_nonce(from).await.ok()),
            ..Default::default()
        };

        let mut tx = EIP1559Transaction {
            to,
            data,
            value: overrides.value.unwrap_or_default(),
            chain_id: overrides
                .chain_id
                .unwrap_or(self.get_chain_id().await?.as_u64()),
            nonce: overrides.nonce.unwrap_or(self.get_nonce(from).await?),
            max_fee_per_gas: overrides
                .gas_price
                .unwrap_or(self.get_gas_price().await?.as_u64()),
            // Should the max_priority_fee_per_gas be dynamic?
            max_priority_fee_per_gas: 10u64,
            ..Default::default()
        };
        tx.gas_limit = overrides
            .gas_limit
            .unwrap_or(self.estimate_gas(generic_transaction).await?);

        Ok(tx)
    }
}
