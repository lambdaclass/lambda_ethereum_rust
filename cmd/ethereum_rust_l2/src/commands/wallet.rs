use crate::{commands::utils::encode_calldata, config::EthereumRustL2Config};
use bytes::Bytes;
use clap::Subcommand;
use ethereum_rust_core::types::{
    EIP1559Transaction, GenericTransaction, PrivilegedL2Transaction, PrivilegedTxType, Transaction,
    TxKind,
};
use ethereum_rust_l2::utils::{
    eth_client::{errors::EthClientError, EthClient},
    merkle_tree::merkle_proof,
};
use ethereum_rust_rlp::encode::RLPEncode;
use ethereum_types::{Address, H256, U256};
use eyre::OptionExt;
use hex::FromHexError;
use keccak_hash::keccak;

const CLAIM_WITHDRAWAL_SIGNATURE: &str = "claimWithdrawal(bytes32 l2WithdrawalTxHash, uint256 claimedAmount, uint256 l2WithdrawalBlockNumber, bytes32[] calldata withdrawalProof)";

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
    ClaimWithdraw {
        #[clap(long = "hash")]
        l2_withdrawal_tx_hash: H256,
        #[clap(long = "amount", value_parser = U256::from_dec_str)]
        claimed_amount: U256,
        #[clap(long = "block-number", value_parser = U256::from_dec_str)]
        withdrawal_l2_block_number: U256,
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
        #[clap(long = "to")]
        to: Option<Address>,
        #[clap(long = "nonce")]
        nonce: Option<u64>,
        #[clap(
            long = "token",
            help = "Specify the token address, the base token is used as default."
        )]
        token_address: Option<Address>,
        #[clap(long, short = 'e', required = false)]
        explorer_url: bool,
    },
    #[clap(about = "Get the withdrawal merkle proof of a transaction.")]
    WithdrawalProof {
        #[clap(long = "hash")]
        tx_hash: H256,
    },
    #[clap(about = "Get the wallet address.")]
    Address,
    #[clap(about = "Get the wallet private key.")]
    PrivateKey,
    #[clap(about = "Send a transaction")]
    Send {
        #[clap(long = "to")]
        to: Address,
        #[clap(
            long = "value",
            value_parser = U256::from_dec_str,
            default_value = "0",
            required = false,
            help = "Value to send in wei"
        )]
        value: U256,
        #[clap(long = "calldata", value_parser = decode_hex, required = false, default_value = "")]
        calldata: Bytes,
        #[clap(
            long = "l1",
            required = false,
            help = "If set it will do an L1 transfer, defaults to an L2 transfer"
        )]
        l1: bool,
        #[clap(long = "chain-id", required = false)]
        chain_id: Option<u64>,
        #[clap(long = "nonce", required = false)]
        nonce: Option<u64>,
        #[clap(long = "gas-limit", required = false)]
        gas_limit: Option<u64>,
        #[clap(long = "gas-price", required = false)]
        gas_price: Option<u64>,
        #[clap(long = "priority-gas-price", required = false)]
        priority_gas_price: Option<u64>,
    },
    #[clap(about = "Make a call to a contract")]
    Call {
        #[clap(long = "to")]
        to: Address,
        #[clap(long = "calldata", value_parser = decode_hex, required = false, default_value = "")]
        calldata: Bytes,
        #[clap(
            long = "l1",
            required = false,
            help = "If set it will do an L1 transfer, defaults to an L2 transfer"
        )]
        l1: bool,
        #[clap(
            long = "value",
            value_parser = U256::from_dec_str,
            default_value = "0",
            required = false,
            help = "Value to send in wei"
        )]
        value: U256,
        #[clap(long = "from", required = false)]
        from: Option<Address>,
        #[clap(long = "gas-limit", required = false)]
        gas_limit: Option<u64>,
        #[clap(long = "gas-price", required = false)]
        gas_price: Option<u64>,
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
        #[clap(
            long = "value",
            value_parser = U256::from_dec_str,
            default_value = "0",
            required = false,
            help = "Value to send in wei"
        )]
        value: U256,
        #[clap(long = "chain-id", required = false)]
        chain_id: Option<u64>,
        #[clap(long = "nonce", required = false)]
        nonce: Option<u64>,
        #[clap(long = "gas-limit", required = false)]
        gas_limit: Option<u64>,
        #[clap(long = "gas-price", required = false)]
        gas_price: Option<u64>,
        #[clap(long = "priority-gas-price", required = false)]
        priority_gas_price: Option<u64>,
    },
}

fn decode_hex(s: &str) -> Result<Bytes, FromHexError> {
    if s.starts_with("0x") {
        return hex::decode(&s[2..]).map(Into::into);
    }
    return hex::decode(s).map(Into::into);
}

async fn make_eip1559_transaction(
    client: &EthClient,
    to: TxKind,
    from: Address,
    data: Bytes,
    value: U256,
    chain_id: u64,
    nonce: Option<u64>,
    gas_limit: Option<u64>,
    gas_price: Option<u64>,
    priority_gas_price: Option<u64>,
) -> Result<EIP1559Transaction, EthClientError> {
    let mut tx = EIP1559Transaction {
        to,
        data,
        value,
        chain_id,
        nonce: match nonce {
            Some(nonce) => nonce,
            None => client.get_nonce(from).await?,
        },
        max_fee_per_gas: match gas_price {
            Some(price) => price,
            None => client.get_gas_price().await?.as_u64(),
        },
        max_priority_fee_per_gas: priority_gas_price.unwrap_or(Default::default()),
        ..Default::default()
    };
    tx.gas_limit = match gas_limit {
        Some(gas) => gas,
        None => client.estimate_gas(tx.clone().into()).await?,
    };
    Ok(tx)
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
            Command::ClaimWithdraw {
                l2_withdrawal_tx_hash,
                claimed_amount,
                withdrawal_l2_block_number,
            } => {
                let claim_withdrawal_data = encode_calldata(
                    CLAIM_WITHDRAWAL_SIGNATURE,
                    &format!("{l2_withdrawal_tx_hash:#x} {claimed_amount} {withdrawal_l2_block_number} {l2_withdrawal_tx_hash}"),
                    false
                )?;

                println!("{}", hex::encode(&claim_withdrawal_data));

                let tx = make_eip1559_transaction(
                    &eth_client,
                    TxKind::Call(cfg.contracts.common_bridge),
                    from,
                    claim_withdrawal_data.into(),
                    U256::from(0),
                    cfg.network.l1_chain_id,
                    None,
                    None,
                    None,
                    None,
                )
                .await?;

                let tx_hash = eth_client
                    .send_eip1559_transaction(tx, cfg.wallet.private_key)
                    .await?;

                println!("Withdrawal claim sent: {tx_hash:#x}");
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
                amount,
                to,
                nonce,
                token_address: _,
                explorer_url: _,
            } => {
                let withdraw_transaction = PrivilegedL2Transaction {
                    to: TxKind::Call(to.unwrap_or(cfg.wallet.address)),
                    value: amount,
                    chain_id: cfg.network.l2_chain_id,
                    nonce: nonce.unwrap_or(rollup_client.get_nonce(from).await?),
                    max_fee_per_gas: 800000000,
                    tx_type: PrivilegedTxType::Withdrawal,
                    gas_limit: 21000 * 2,
                    ..Default::default()
                };

                let tx_hash = rollup_client
                    .send_privileged_l2_transaction(withdraw_transaction, cfg.wallet.private_key)
                    .await?;

                println!("Withdrawal sent: {tx_hash:#x}");
            }
            Command::WithdrawalProof { tx_hash } => {
                let tx_receipt = rollup_client
                    .get_transaction_receipt(tx_hash)
                    .await?
                    .ok_or_eyre("Transaction receipt not found")?;

                let transactions = rollup_client
                    .get_block_by_hash(tx_receipt.block_info.block_hash)
                    .await?
                    .transactions;

                let tx_withdrawal_hash = transactions
                    .iter()
                    .find_map(|tx| {
                        if tx.compute_hash() != tx_hash {
                            return None;
                        }
                        match tx {
                            Transaction::PrivilegedL2Transaction(tx) => {
                                Some(tx.get_withdrawal_hash())
                            }
                            _ => None,
                        }
                    })
                    .flatten()
                    .ok_or_eyre("Transaction is not a Withdrawal")?;

                let path = merkle_proof(
                    transactions
                        .iter()
                        .filter_map(|tx| match tx {
                            Transaction::PrivilegedL2Transaction(tx) => tx.get_withdrawal_hash(),
                            _ => None,
                        })
                        .collect(),
                    tx_withdrawal_hash,
                )
                .ok_or_eyre(
                    "Transaction's WithdrawalData is not in block's WithdrawalDataMerkleRoot",
                )?;
                println!("{path:?}");
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
                chain_id,
                nonce,
                gas_limit,
                gas_price,
                priority_gas_price,
            } => {
                let client = match l1 {
                    true => eth_client,
                    false => rollup_client,
                };

                let tx = make_eip1559_transaction(
                    &client,
                    TxKind::Call(to),
                    from,
                    calldata,
                    value,
                    chain_id.unwrap_or_else(|| match l1 {
                        true => cfg.network.l1_chain_id,
                        false => cfg.network.l2_chain_id,
                    }),
                    nonce,
                    gas_limit,
                    gas_price,
                    priority_gas_price,
                )
                .await?;

                let tx_hash = client
                    .send_eip1559_transaction(tx, cfg.wallet.private_key)
                    .await?;

                println!(
                    "[{}] Transaction sent: {tx_hash:#x}",
                    if l1 { "L1" } else { "L2" }
                );
            }
            Command::Call {
                to,
                calldata,
                l1,
                value,
                from,
                gas_limit,
                gas_price,
            } => {
                let client = match l1 {
                    true => eth_client,
                    false => rollup_client,
                };

                let call_tx = GenericTransaction {
                    to: TxKind::Call(to),
                    input: calldata,
                    value,
                    from: from.unwrap_or(Default::default()),
                    gas: gas_limit,
                    gas_price: gas_price.unwrap_or(Default::default()),
                    ..Default::default()
                };

                let result = client.call(call_tx).await?;

                println!("{result}");
            }
            Command::Deploy {
                bytecode,
                l1,
                value,
                chain_id,
                nonce,
                gas_limit,
                gas_price,
                priority_gas_price,
            } => {
                let client = match l1 {
                    true => eth_client,
                    false => rollup_client,
                };

                let nonce = nonce.unwrap_or(client.get_nonce(from).await?);
                let tx = make_eip1559_transaction(
                    &client,
                    TxKind::Create,
                    from,
                    bytecode,
                    value,
                    chain_id.unwrap_or_else(|| match l1 {
                        true => cfg.network.l1_chain_id,
                        false => cfg.network.l2_chain_id,
                    }),
                    Some(nonce),
                    gas_limit,
                    gas_price,
                    priority_gas_price,
                )
                .await?;

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
