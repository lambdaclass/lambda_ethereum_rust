use std::time::Duration;

use tokio::time::sleep;

pub struct Prover {}

impl Prover {
    pub async fn prove(_id: u32) {
        sleep(Duration::from_secs(5)).await;
    }
}
