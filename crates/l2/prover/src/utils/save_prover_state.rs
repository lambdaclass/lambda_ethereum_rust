use directories::ProjectDirs;
use ethereum_rust_core::types::BlockHeader;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::{
    fs::create_dir_all,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ProverState {
    pub block_header: BlockHeader,
}

fn create_prover_state_file_path(file_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let project_dir = ProjectDirs::from("", "", "ethereum_rust_l2")
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Couldn't get project_dir."))?;

    let binding = project_dir.data_local_dir().join(file_name);
    let path_str = binding
        .to_str()
        .ok_or_else(|| Box::<dyn std::error::Error>::from("Couldn't convert path to str."))?;

    Ok(path_str.to_string())
}

pub fn get_default_prover_state_file_path() -> Result<String, Box<dyn std::error::Error>> {
    create_prover_state_file_path("prover_client_state.json")
}

pub fn create_prover_state_file() -> Result<File, Box<dyn std::error::Error>> {
    let file_path = get_default_prover_state_file_path()?;
    if let Some(parent) = Path::new(&file_path).parent() {
        create_dir_all(parent)?;
    }
    File::create(file_path).map_err(Into::into)
}

pub fn persist_block_in_prover_state(
    file_path: &str,
    block_header: BlockHeader,
) -> Result<(), Box<dyn std::error::Error>> {
    let inner = File::create(file_path)?;
    let mut writer = BufWriter::new(inner);

    let prover_state = ProverState { block_header };
    serde_json::to_writer(&mut writer, &prover_state)?;
    writer.flush()?;

    Ok(())
}

pub fn read_block_in_prover_state(
    file_path: &str,
) -> Result<ProverState, Box<dyn std::error::Error>> {
    let inner = File::open(file_path)?;
    let mut reader = BufReader::new(inner);
    let mut buf = String::new();

    reader.read_to_string(&mut buf)?;

    let prover_state: ProverState = serde_json::from_str(&buf)?;

    Ok(prover_state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum_rust_l2::utils::test_data_io;
    use std::fs::{self, create_dir_all};
    use std::path::{Path, PathBuf};

    #[test]
    fn test_prover_state_file_integration() -> Result<(), Box<dyn std::error::Error>> {
        let file_path = create_prover_state_file_path("test_prover_state.json")?;
        if let Some(parent) = Path::new(&file_path).parent() {
            create_dir_all(parent)?;
        }

        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // Go back 5 levels (Go to the root of the project)
        for _ in 0..5 {
            path.pop();
        }
        path.push("test_data");

        println!("path {path:?}");

        let chain_file_path = path.join("chain.rlp");

        let blocks = test_data_io::read_chain_file(chain_file_path.to_str().unwrap());
        let last_block = blocks.last().unwrap().clone();

        persist_block_in_prover_state(&file_path, last_block.header.clone())?;

        let prover_state = read_block_in_prover_state(&file_path)?;

        assert_eq!(
            prover_state.block_header.compute_block_hash(),
            last_block.hash()
        );

        fs::remove_file(file_path)?;

        Ok(())
    }
}
