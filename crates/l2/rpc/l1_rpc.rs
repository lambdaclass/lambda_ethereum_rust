use ethereum_types::{Address, H256, U256};
use reqwest::Client;

#[derive(Debug, serde::Deserialize)]
pub struct Error {
    code: i128,
    message: String,
    data: Option<String>,
}

#[derive(serde::Deserialize)]
struct Response<T> {
    id: i32,
    jsonrpc: String,
    result: Option<T>,
    error: Option<Error>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Log {
    address: String,
    topics: Vec<String>,
    data: String,
    #[serde(rename = "blockNumber")]
    block_number: String,
    #[serde(rename = "transactionHash")]
    transaction_hash: String,
    #[serde(rename = "transactionIndex")]
    transaction_index: String,
    #[serde(rename = "blockHash")]
    block_hash: String,
    #[serde(rename = "logIndex")]
    log_index: String,
    removed: bool,
}

pub struct L1Rpc {
    client: Client,
    url: String,
}

impl L1Rpc {
    pub fn new(url: &str) -> Self {
        Self {
            client: Client::new(),
            url: url.to_string(),
        }
    }

    async fn send_request(
        &self,
        method: &str,
        params: Option<&str>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .post(&self.url)
            .header("content-type", "application/json")
            .body(
                r#"{"jsonrpc":"2.0","method":""#.to_string()
                    + method
                    + r#"","params":"#
                    + params.unwrap_or("[]")
                    + r#","id":1}"#,
            )
            .send()
            .await
    }

    pub async fn get_block_number(&self) -> Result<U256, String> {
        match self.send_request("eth_blockNumber", None).await {
            Ok(res) => match res.json::<Response<String>>().await {
                Ok(body) => {
                    if let Some(error) = body.error {
                        return Err(error.message);
                    }
                    return Ok(body.result.unwrap().parse().unwrap());
                }
                Err(e) => Err(e.to_string()),
            },
            Err(e) => Err(e.to_string()),
        }
    }

    pub async fn get_logs(
        &self,
        from_block: U256,
        to_block: U256,
        address: Address,
        topic: H256,
    ) -> Result<Vec<Log>, String> {
        let params = format!(
            r#"[{{"fromBlock": "{:#x}", "toBlock": "{:#x}", "address": "{:#x}", "topics": ["{:#x}"]}}]"#,
            from_block, to_block, address, topic
        );

        match self.send_request("eth_getLogs", Some(&params)).await {
            Ok(res) => match res.json::<Response<Vec<Log>>>().await {
                Ok(body) => {
                    if let Some(error) = body.error {
                        return Err(error.message);
                    }
                    return Ok(body.result.unwrap());
                }
                Err(e) => Err(e.to_string()),
            },
            Err(e) => Err(e.to_string()),
        }
    }
}
