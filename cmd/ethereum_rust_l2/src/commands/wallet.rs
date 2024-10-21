use crate::config::EthereumRustL2Config;
use bytes::Bytes;
use clap::Subcommand;
use ethereum_rust_core::types::{EIP1559Transaction, TxKind};
use ethereum_rust_l2::utils::eth_client::EthClient;
use ethereum_rust_rlp::encode::RLPEncode;
use ethereum_types::{Address, H256, U256};
use hex::FromHexError;
use keccak_hash::keccak;

#[derive(Subcommand)]
pub(crate) enum Command {
    #[clap(about = "Get the balance of the wallet.")]
    Balance {
        #[clap(long = "token")]
        token_address: Option<Address>,
        #[arg(long = "l2", required = false)]
        l2: bool,
        #[arg(long = "l1", required = false)]
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
    #[clap(about = "Send a transaction")]
    Send {
        #[clap(long = "to")]
        to: Address,
        #[clap(long = "value", value_parser = U256::from_dec_str, default_value = "0", required = false)]
        value: U256,
        #[clap(long = "calldata", value_parser = decode_hex, required = false, default_value = "")]
        calldata: Bytes,
        #[clap(
            long = "l1",
            required = false,
            help = "If set it will do an L1 transfer, defaults to an L2 transfer"
        )]
        l1: bool,
    },
    #[clap(about = "Deploy a contract")]
    Deploy {
        #[clap(long = "bytecode", value_parser = decode_hex)]
        bytecode: Bytes,
        #[clap(
            long = "l1",
            required = false,
            help = "If set it will do an L1 transfer, defaults to an L2 transfer"
        )]
        l1: bool,
    },
}

pub fn decode_hex(s: &str) -> Result<Bytes, FromHexError> {
    if s.starts_with("0x") {
        return hex::decode(&s[2..]).map(Into::into);
    }
    return hex::decode(s).map(Into::into);
}

impl Command {
    pub async fn run(self, cfg: EthereumRustL2Config) -> eyre::Result<()> {
        let eth_client = EthClient::new(&cfg.network.l1_rpc_url);
        let rollup_client = EthClient::new(&cfg.network.l2_rpc_url);
        let from = cfg.wallet.address;
        match self {
            Command::Balance {
                token_address,
                l2,
                l1,
            } => {
                if token_address.is_some() {
                    todo!("Handle ERC20 balances")
                }
                if !l1 || l2 {
                    let account_balance = rollup_client.get_balance(from).await?;
                    println!("[L2] Account balance: {account_balance}");
                }
                if l1 {
                    let account_balance = eth_client.get_balance(from).await?;
                    println!("[L1] Account balance: {account_balance}");
                }
            }
            Command::Deposit {
                amount,
                token_address,
                to,
                explorer_url: _,
            } => {
                if to.is_some() {
                    // There are two ways of depositing funds into the L2:
                    // 1. Directly transferring funds to the bridge.
                    // 2. Depositing through a contract call to the deposit method of the bridge.
                    // The second method is not handled in the CLI yet.
                    todo!("Handle deposits through contract")
                }
                if token_address.is_some() {
                    todo!("Handle ERC20 deposits")
                }
                Box::pin(async {
                    Self::Transfer {
                        amount,
                        token_address: None,
                        to: cfg.contracts.common_bridge,
                        l1: true,
                        explorer_url: false,
                    }
                    .run(cfg)
                    .await
                })
                .await?;
            }
            Command::FinalizeWithdraw {
                l2_withdrawal_tx_hash: _,
            } => {
                todo!()
            }
            Command::Transfer {
                amount,
                token_address,
                to,
                l1,
                explorer_url: _,
            } => {
                if token_address.is_some() {
                    todo!("Handle ERC20 transfers")
                }

                let mut transfer_transaction = EIP1559Transaction {
                    to: TxKind::Call(to),
                    value: amount,
                    chain_id: cfg.network.l1_chain_id,
                    nonce: eth_client.get_nonce(from).await?,
                    max_fee_per_gas: eth_client.get_gas_price().await?.as_u64(),
                    ..Default::default()
                };

                // let estimated_gas = eth_client
                //     .estimate_gas(transfer_transaction.clone())
                //     .await?;

                transfer_transaction.gas_limit = 21000 * 2;

                let tx_hash = if l1 {
                    eth_client
                        .send_eip1559_transaction(transfer_transaction, cfg.wallet.private_key)
                        .await?
                } else {
                    rollup_client
                        .send_eip1559_transaction(transfer_transaction, cfg.wallet.private_key)
                        .await?
                };

                println!(
                    "[{}] Transfer sent: {tx_hash:#x}",
                    if l1 { "L1" } else { "L2" }
                );
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
            Command::Send {
                to,
                value,
                calldata,
                l1,
            } => {
                let client = match l1 {
                    true => eth_client,
                    false => rollup_client,
                };

                let mut tx = EIP1559Transaction {
                    to: TxKind::Call(to),
                    value,
                    chain_id: match l1 {
                        true => cfg.network.l1_chain_id,
                        false => cfg.network.l2_chain_id,
                    },
                    data: calldata,
                    nonce: client.get_nonce(from).await?,
                    max_fee_per_gas: client
                        .get_gas_price()
                        .await?
                        .as_u64()
                        // TODO: Check this multiplier. Without it, the transaction
                        // fail with error "gas price is less than basefee"
                        .saturating_mul(10000000),
                    ..Default::default()
                };
                tx.gas_limit = client.estimate_gas(tx.clone()).await?;

                let tx_hash = client
                    .send_eip1559_transaction(tx, cfg.wallet.private_key)
                    .await?;

                println!(
                    "[{}] Transaction sent: {tx_hash:#x}",
                    if l1 { "L1" } else { "L2" }
                );
            }
            Command::Deploy { bytecode, l1 } => {
                let client = match l1 {
                    true => eth_client,
                    false => rollup_client,
                };

                let nonce = client.get_nonce(from).await?;
                let mut tx = EIP1559Transaction {
                    to: TxKind::Create,
                    chain_id: match l1 {
                        true => cfg.network.l1_chain_id,
                        false => cfg.network.l2_chain_id,
                    },
                    nonce,
                    max_fee_per_gas: client
                        .get_gas_price()
                        .await?
                        .as_u64()
                        // TODO: Check this multiplier. Without it, the transaction
                        // fail with error "gas price is less than basefee"
                        .saturating_mul(10000000),
                    data: bytecode.into(),
                    ..Default::default()
                };

                tx.gas_limit = client.estimate_gas(tx.clone()).await?.saturating_mul(500);
                tx.gas_limit = 10000000;
                tx.max_fee_per_gas = 3000011820;

                let hash = client
                    .send_eip1559_transaction(tx, cfg.wallet.private_key)
                    .await?;

                let encoded_from = from.encode_to_vec();
                let encoded_nonce = nonce.encode_to_vec();
                let mut encoded = vec![(0xc0 + encoded_from.len() + encoded_nonce.len()) as u8];
                encoded.extend(encoded_from.clone());
                encoded.extend(encoded_nonce.clone());
                let deployed_address = Address::from(keccak(encoded));

                println!("Contract deployed in tx: {hash:#x}");
                println!("Contract address: {deployed_address:#x}");
            }
        };
        Ok(())
    }
}
