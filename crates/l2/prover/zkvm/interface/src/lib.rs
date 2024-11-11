pub mod methods {
    #[cfg(any(clippy, not(feature = "build_zkvm")))]
    pub const ZKVM_PROGRAM_ELF: &[u8] = &[0];
    #[cfg(any(clippy, not(feature = "build_zkvm")))]
    pub const ZKVM_PROGRAM_ID: [u32; 8] = [0_u32; 8];

    #[cfg(all(not(clippy), feature = "build_zkvm"))]
    include!(concat!(env!("OUT_DIR"), "/methods.rs"));
}

pub mod io {
    use ethereum_rust_core::{
        types::{Block, BlockHeader},
        H256,
    };
    use ethereum_rust_vm::execution_db::ExecutionDB;
    use serde::{Deserialize, Serialize};

    /// Private input variables passed into the zkVM execution program.
    #[derive(Serialize, Deserialize)]
    pub struct ProgramInput {
        /// block to execute
        pub block: Block,
        /// header of the previous block
        pub parent_block_header: BlockHeader,
        /// database containing only the data necessary to execute
        pub db: ExecutionDB,
    }

    /// Public output variables exposed by the zkVM execution program. Some of these are part of
    /// the program input.
    #[derive(Serialize, Deserialize)]
    pub struct ProgramOutput {
        /// initial state trie root hash
        pub initial_state_hash: H256,
        /// final state trie root hash
        pub final_state_hash: H256,
    }
}
