use ethrex_core::{
    types::{AccountState, Block},
    Address,
};
use ethrex_rlp::decode::RLPDecode;

use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct BlockResponse {
    result: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountProofResponse {
    balance: String,
    code_hash: String,
    nonce: String,
    storage_hash: String,
}

pub async fn get_block(rpc_url: String, block_number: usize) -> Result<Block, String> {
    let client = reqwest::Client::new();

    let block_number = format!("0x{block_number:x}");
    let request = &json!({
        "id": 1,
        "jsonrpc": "2.0",
        "method": "debug_getRawBlock",
        "params": [block_number]
    });

    let response = client
        .post(rpc_url)
        .json(request)
        .send()
        .await
        .map_err(|err| err.to_string())?;

    let hex_encoded_block = response
        .json::<BlockResponse>()
        .await
        .map_err(|err| err.to_string())?;
    let encoded_block = hex::decode(hex_encoded_block.result.trim_start_matches("0x"))
        .map_err(|err| err.to_string())?;

    Ok(Block::decode_unfinished(&encoded_block)
        .map_err(|err| err.to_string())?
        .0)
}

pub async fn get_account(
    rpc_url: String,
    block_number: usize,
    address: Address,
) -> Result<AccountState, String> {
    let client = reqwest::Client::new();

    let block_number = format!("0x{block_number:x}");
    let address = format!("0x{address:x}");

    let request = &json!(
           {
               "id": 1,
               "jsonrpc": "2.0",
               "method": "eth_getProof",
               "params":[address, [], block_number]
           }
    );
    let response = client
        .post(rpc_url)
        .json(request)
        .send()
        .await
        .map_err(|err| err.to_string())?;

    let account_proof = response
        .json::<AccountProofResponse>()
        .await
        .map_err(|err| err.to_string())?;

    Ok(AccountState {
        nonce: account_proof
            .nonce
            .parse()
            .map_err(|_| "failed to parse nonce".to_string())?,
        balance: account_proof
            .balance
            .parse()
            .map_err(|_| "failed to parse balance".to_string())?,
        storage_root: account_proof
            .storage_hash
            .parse()
            .map_err(|_| "failed to parse storage root".to_string())?,
        code_hash: account_proof
            .code_hash
            .parse()
            .map_err(|_| "failed to parse code hash".to_string())?,
    })
}
