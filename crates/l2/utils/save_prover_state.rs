use directories::ProjectDirs;
use ethereum_rust_storage::AccountUpdate;
use serde::{Deserialize, Serialize};
use std::fs::{create_dir, File};
use std::path::PathBuf;
use std::{
    fs::create_dir_all,
    io::{BufWriter, Write},
};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum StateFileType {
    Proof,
    AccountUpdates,
}

const DEFAULT_DATADIR: &str = "ethereum_rust_l2";

#[inline(always)]
fn default_datadir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    create_datadir(DEFAULT_DATADIR)
}

#[inline(always)]
fn create_datadir(dir_name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path_buf_data_dir = ProjectDirs::from("", "", dir_name)
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Couldn't get project_dir."))?
        .data_local_dir()
        .to_path_buf();
    Ok(path_buf_data_dir)
}

#[inline(always)]
fn get_state_dir_for_block(block_number: u64) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path_buf = default_datadir()?;
    path_buf.push(block_number.to_string());

    Ok(path_buf)
}

fn create_state_file_for_block_number(
    block_number: u64,
    state_file_type: StateFileType,
) -> Result<File, Box<dyn std::error::Error>> {
    let path_buf = get_state_dir_for_block(block_number)?;
    if let Some(parent) = path_buf.parent() {
        if let Err(e) = create_dir_all(parent) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                eprintln!("Directory already exists: {:?}", parent);
                return Err(e.into());
            }
        }
    }

    let block_number = path_buf
        .file_name()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Error: No file_name()"))?
        .to_string_lossy();

    let block_number = block_number.parse::<u64>()?;

    let file_path = match state_file_type {
        StateFileType::AccountUpdates => {
            path_buf.join(format!("account_updates{block_number}.json"))
        }
        StateFileType::Proof => path_buf.join(format!("proof_{block_number}.json")),
    };

    if let Err(e) = create_dir(&path_buf) {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(e.into());
        }
        eprintln!("Directory already exists: {:?}", path_buf);
    }

    File::create(file_path).map_err(Into::into)
}

pub fn persist_state_in_block_state_path(
    block_number: u64,
    state_file_type: StateFileType,
    proof: Option<&ethereum_rust_core::H256>,
    account_updates: Option<&Vec<AccountUpdate>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let inner = create_state_file_for_block_number(block_number, state_file_type)?;

    match state_file_type {
        StateFileType::Proof => {
            let value = proof
                .ok_or_else(|| Box::<dyn std::error::Error>::from("Error: proof not present"))?;
            let mut writer = BufWriter::new(inner);
            serde_json::to_writer(&mut writer, value)?;
            writer.flush()?;
        }
        StateFileType::AccountUpdates => {
            let value = account_updates.ok_or_else(|| {
                Box::<dyn std::error::Error>::from("Error: account_updates not present")
            })?;
            let mut writer = BufWriter::new(inner);
            serde_json::to_writer(&mut writer, value)?;
            writer.flush()?;
        }
    };

    Ok(())
}

fn get_latest_block_state_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let data_dir = default_datadir()?;
    let latest_block_number = std::fs::read_dir(&data_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_dir() {
                path.file_name()?.to_str()?.parse::<u64>().ok()
            } else {
                None
            }
        })
        .max();

    match latest_block_number {
        Some(block_number) => {
            let latest_path = data_dir.join(block_number.to_string());
            Ok(latest_path)
        }
        None => Err(Box::from("No valid block directories found")),
    }
}

#[cfg(test)]
mod tests {
    use ethereum_rust_blockchain::add_block;
    use ethereum_rust_storage::{EngineType, Store};
    use ethereum_rust_vm::execution_db::ExecutionDB;

    use super::*;
    use crate::utils::test_data_io;
    use std::fs::{self};

    #[test]
    fn test_state_file_integration() -> Result<(), Box<dyn std::error::Error>> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // Go back 4 levels (Go to the root of the project)
        for _ in 0..4 {
            path.pop();
        }
        path.push("test_data");

        let chain_file_path = path.join("l2-loadtest.rlp");
        let genesis_file_path = path.join("genesis-l2.json");

        let store = Store::new("memory", EngineType::InMemory).expect("Failed to create Store");

        let genesis = test_data_io::read_genesis_file(genesis_file_path.to_str().unwrap());
        store.add_initial_state(genesis.clone()).unwrap();

        let blocks = test_data_io::read_chain_file(chain_file_path.to_str().unwrap());
        for block in &blocks {
            add_block(block, &store).unwrap();
        }

        let mut account_updates_vec: Vec<AccountUpdate> = Vec::new();

        for block in &blocks {
            let (_, account_updates) =
                ExecutionDB::from_exec(blocks.last().unwrap(), &store).unwrap();

            account_updates_vec = account_updates;

            persist_state_in_block_state_path(
                block.header.number,
                StateFileType::AccountUpdates,
                None,
                Some(&account_updates_vec),
            )?;
        }

        let latest_block_path = get_latest_block_state_path()?;

        assert_eq!(
            latest_block_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .parse::<u64>()?,
            blocks.len() as u64
        );

        // Read account_updates back todo

        fs::remove_dir_all(default_datadir()?)?;

        Ok(())
    }
}
