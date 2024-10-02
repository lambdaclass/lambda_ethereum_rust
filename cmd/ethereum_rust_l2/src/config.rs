use crate::{
    commands,
    utils::{
        config::{default_values::DEFAULT_CONFIG_NAME, prompt, selected_config_path},
        messages::CONFIG_CREATE_NAME_PROMPT_MSG,
    },
};
use ethereum_types::Address;
use eyre::Context;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct EthereumRustL2Config {
    pub network: NetworkConfig,
    pub wallet: WalletConfig,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct NetworkConfig {
    pub l1_rpc_url: String,
    pub l1_chain_id: u64,
    pub l1_explorer_url: String,
    pub l2_rpc_url: String,
    pub l2_chain_id: u64,
    pub l2_explorer_url: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct WalletConfig {
    pub address: Address,
    pub private_key: String,
}

pub async fn try_load_selected_config() -> eyre::Result<Option<EthereumRustL2Config>> {
    let config_path = selected_config_path()?;
    if !config_path.exists() {
        return Ok(None);
    }
    let config = std::fs::read_to_string(config_path).context("Failed to read config file")?;
    toml::from_str(&config)
        .context("Failed to parse config file")
        .map(Some)
}

pub async fn load_selected_config() -> eyre::Result<EthereumRustL2Config> {
    let config_path = selected_config_path()?;
    if !config_path.exists() {
        println!("No config set, please select a config to set");
        if (commands::config::Command::Set { config_name: None })
            .run()
            .await
            .is_err()
        {
            let config_name = prompt(
                CONFIG_CREATE_NAME_PROMPT_MSG,
                DEFAULT_CONFIG_NAME.to_owned(),
            )?
            .to_owned();
            commands::config::Command::Create { config_name }
                .run()
                .await?;
        }
    }
    let config = std::fs::read_to_string(config_path).context("Failed to read config file")?;
    toml::from_str(&config).context("Failed to parse config file")
}
