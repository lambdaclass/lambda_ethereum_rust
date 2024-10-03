use crate::config::EthereumRustL2Config;
use clap::Subcommand;
use ethereum_types::{Address, H256, U256};

#[derive(Subcommand)]
pub(crate) enum Command {
    #[clap(about = "Get the balance of the wallet.")]
    Balance {
        #[clap(long = "token")]
        token_address: Option<Address>,
        #[clap(long = "l2", required = false)]
        l2: bool,
        #[clap(long = "l1", required = false)]
        l1: bool,
    },
    #[clap(about = "Deposit funds into some wallet.")]
    Deposit {
        // TODO: Parse ether instead.
        #[clap(long = "amount", value_parser = U256::from_dec_str)]
        amount: U256,
        #[clap(
            long = "token",
            help = "Specify the token address, the base token is used as default."
        )]
        token_address: Option<Address>,
        #[clap(
            long = "to",
            help = "Specify the wallet in which you want to deposit your funds."
        )]
        to: Option<Address>,
        #[clap(long, short = 'e', required = false)]
        explorer_url: bool,
    },
    #[clap(about = "Finalize a pending withdrawal.")]
    FinalizeWithdraw {
        #[clap(long = "hash")]
        l2_withdrawal_tx_hash: H256,
    },
    #[clap(about = "Transfer funds to another wallet.")]
    Transfer {
        // TODO: Parse ether instead.
        #[clap(long = "amount", value_parser = U256::from_dec_str)]
        amount: U256,
        #[clap(long = "token")]
        token_address: Option<Address>,
        #[clap(long = "to")]
        to: Address,
        #[clap(
            long = "l1",
            required = false,
            help = "If set it will do an L1 transfer, defaults to an L2 transfer"
        )]
        l1: bool,
        #[clap(long, short = 'e', required = false)]
        explorer_url: bool,
    },
    #[clap(about = "Withdraw funds from the wallet.")]
    Withdraw {
        // TODO: Parse ether instead.
        #[clap(long = "amount", value_parser = U256::from_dec_str)]
        amount: U256,
        #[clap(
            long = "token",
            help = "Specify the token address, the base token is used as default."
        )]
        token_address: Option<Address>,
        #[clap(long, short = 'e', required = false)]
        explorer_url: bool,
    },
    #[clap(about = "Get the wallet address.")]
    Address,
    #[clap(about = "Get the wallet private key.")]
    PrivateKey,
}

impl Command {
    pub async fn run(self, _cfg: EthereumRustL2Config) -> eyre::Result<()> {
        match self {
            Command::Balance {
                token_address: _,
                l2: _,
                l1: _,
            } => {
                todo!()
            }
            Command::Deposit {
                amount: _,
                token_address: _,
                to: _,
                explorer_url: _,
            } => {
                todo!()
            }
            Command::FinalizeWithdraw {
                l2_withdrawal_tx_hash: _,
            } => {
                todo!()
            }
            Command::Transfer {
                amount: _,
                token_address: _,
                to: _,
                l1: _,
                explorer_url: _,
            } => {
                todo!()
            }
            Command::Withdraw {
                amount: _,
                token_address: _,
                explorer_url: _,
            } => {
                todo!()
            }
            Command::Address => {
                todo!()
            }
            Command::PrivateKey => {
                todo!()
            }
        };
    }
}
