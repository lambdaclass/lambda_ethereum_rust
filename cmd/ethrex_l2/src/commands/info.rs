use crate::config::EthrexL2Config;
use clap::Subcommand;
use colored::{self, Colorize};
use ethrex_l2::utils::eth_client::EthClient;

#[derive(Subcommand)]
pub(crate) enum Command {
    #[clap(about = "Get latestCommittedBlock and latestVerifiedBlock from the OnChainProposer.")]
    LatestBlocks,
}

impl Command {
    pub async fn run(self, cfg: EthrexL2Config) -> eyre::Result<()> {
        let eth_client = EthClient::new(&cfg.network.l1_rpc_url);
        let _rollup_client = EthClient::new(&cfg.network.l2_rpc_url);
        let on_chain_proposer_address = cfg.contracts.on_chain_proposer;
        match self {
            Command::LatestBlocks => {
                let last_committed_block =
                    EthClient::get_last_committed_block(&eth_client, on_chain_proposer_address)
                        .await?;

                let last_verified_block =
                    EthClient::get_last_verified_block(&eth_client, on_chain_proposer_address)
                        .await?;

                println!(
                    "latestCommittedBlock: {}",
                    format!("{last_committed_block}").bright_cyan()
                );

                println!(
                    "latestVerifiedBlock: {}",
                    format!("{last_verified_block}").bright_cyan()
                );
            }
        }
        Ok(())
    }
}
