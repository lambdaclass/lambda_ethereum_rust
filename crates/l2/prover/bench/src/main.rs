use bench::get_block;
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    rpc_url: String,
    #[arg(short, long)]
    block_number: usize,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let block = get_block(args.rpc_url, args.block_number)
        .await
        .expect("failed");

    println!("Succesfully fetched block {}", block.hash());
}
