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
    use ethereum_rust_rlp::{decode::RLPDecode, encode::RLPEncode};
    use ethereum_rust_vm::execution_db::ExecutionDB;
    use serde::{Deserialize, Serialize};
    use serde_with::serde_as;

    /// Private input variables passed into the zkVM execution program.
    #[serde_as]
    #[derive(Serialize, Deserialize)]
    pub struct ProgramInput {
        /// block to execute
        #[serde_as(as = "RLPBlock")]
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

    /// Used with [serde_with] to encode a Block into RLP before serializing its bytes. This is
    /// necessary because the [ethereum_rust_core::types::Transaction] type doesn't serializes into any
    /// format other than JSON.
    pub struct RLPBlock;

    impl serde_with::SerializeAs<Block> for RLPBlock {
        fn serialize_as<S>(val: &Block, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut encoded = Vec::new();
            val.encode(&mut encoded);
            serde_with::Bytes::serialize_as(&encoded, serializer)
        }
    }

    impl<'de> serde_with::DeserializeAs<'de, Block> for RLPBlock {
        fn deserialize_as<D>(deserializer: D) -> Result<Block, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let encoded: Vec<u8> = serde_with::Bytes::deserialize_as(deserializer)?;
            Block::decode(&encoded).map_err(serde::de::Error::custom)
        }
    }
}
