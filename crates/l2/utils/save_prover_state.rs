use directories::ProjectDirs;
use ethereum_rust_storage::AccountUpdate;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::PathBuf;
use std::{
    fs::create_dir_all,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum StateFileType {
    Proof,
    AccountUpdates,
}

#[inline(always)]
fn default_datadir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path_buf_data_dir = ProjectDirs::from("", "", "ethereum_rust_l2")
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
        create_dir_all(parent)?;
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
    use super::*;
    use crate::utils::test_data_io;
    use std::fs::{self, create_dir_all};
    use std::path::{Path, PathBuf};

    fn default_datadir_test() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let path_buf_data_dir = ProjectDirs::from("", "", "test_state")
            .ok_or_else(|| Box::<dyn std::error::Error>::from("Couldn't get project_dir."))?
            .data_local_dir()
            .to_path_buf();
        Ok(path_buf_data_dir)
    }

    fn get_state_dir_for_block_test(
        block_number: u64,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut path_buf = default_datadir_test()?;
        path_buf.push(block_number.to_string());

        Ok(path_buf)
    }

    #[test]
    fn test_state_file_integration() -> Result<(), Box<dyn std::error::Error>> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // Go back 4 levels (Go to the root of the project)
        for _ in 0..4 {
            path.pop();
        }
        path.push("test_data");

        let chain_file_path = path.join("chain.rlp");

        let blocks = test_data_io::read_chain_file(chain_file_path.to_str().unwrap());
        let last_block = blocks.last().unwrap().clone();

        fs::remove_dir_all(default_datadir_test()?)?;

        Ok(())
    }
}
