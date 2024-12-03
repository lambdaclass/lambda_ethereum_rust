use bench::rpc::{get_account, get_block};
use clap::Parser;
use ethrex_vm::{execution_db::touched_state::get_touched_state, SpecId};
use futures_util::future::join_all;
use tokio_utils::RateLimiter;

const MAINNET_CHAIN_ID: u64 = 0x1;
const MAINNET_SPEC_ID: SpecId = SpecId::CANCUN;

const RPC_RATE_LIMIT: usize = 100; // requests per second

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    rpc_url: String,
    #[arg(short, long)]
    block_number: usize,
}

#[tokio::main]
async fn main() {
    let Args {
        rpc_url,
        block_number,
    } = Args::parse();

    println!("fetching block {block_number}");
    let block = get_block(&rpc_url, &block_number)
        .await
        .expect("failed to fetch block");

    println!("pre-executing transactions to get touched state keys");
    let touched_state = get_touched_state(&block, MAINNET_CHAIN_ID, MAINNET_SPEC_ID)
        .expect("failed to get touched state");

    println!("fetching touched state values");
    let mut accounts = Vec::with_capacity(touched_state.len());

    let rate_limiter = RateLimiter::new(std::time::Duration::from_secs(1));
    let mut fetched_accs = 0;
    for request_chunk in touched_state.chunks(RPC_RATE_LIMIT) {
        let rate_limited = rate_limiter.throttle(|| async {
            join_all(
                request_chunk
                    .iter()
                    .map(|(address, _)| get_account(&rpc_url, &block_number, address)),
            )
            .await
        });

        let account_chunk = rate_limited
            .await
            .into_iter()
            .collect::<Result<Vec<_>, String>>()
            .expect("failed to fetch accounts");

        accounts.extend(account_chunk);

        fetched_accs += request_chunk.len();
        println!(
            "fetched {} accounts of {}",
            fetched_accs,
            touched_state.len()
        );
    }

    // TODO: storage

    // 4. create prover program input and execute. Measure time.
    // 5. invoke rsp and execute too. Save in cache
}
