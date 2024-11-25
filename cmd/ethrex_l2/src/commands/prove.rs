use clap::Args;
use ethrex_l2::utils::test_data_io::{generate_prover_input, read_chain_file, read_genesis_file};
use ethrex_prover_lib::prover::Prover;
use log::info;

#[derive(Args)]
pub(crate) struct Command {
    #[clap(
        short = 'g',
        long = "genesis",
        help = "Path to the file containing the genesis block."
    )]
    genesis: String,
    #[clap(
        short = 'c',
        long = "chain",
        help = "Path to the file containing the test chain."
    )]
    chain: String,
    #[clap(
        short = 'n',
        long = "block-number",
        help = "Number of the block in the test chain to prove."
    )]
    block_number: usize,
}

impl Command {
    pub fn run(self) -> eyre::Result<()> {
        let genesis = read_genesis_file(&self.genesis);
        let chain = read_chain_file(&self.chain);
        let prover_input_data = generate_prover_input(genesis, chain, self.block_number)?;

        let mut prover = Prover::new();
        prover.set_input(prover_input_data);

        prover.prove().expect("proving failed");

        info!("Successfully proven block");
        Ok(())
    }
}
