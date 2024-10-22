use tracing::info;

// risc0
//use program_interface::{ZKVM_PROGRAM_ELF, ZKVM_PROGRAM_ID};
const ZKVM_PROGRAM_ELF: &[u8] = &[0, 0, 0];
const ZKVM_PROGRAM_ID: [u32; 8] = [0_u32; 8];
use risc0_zkvm::{default_prover, ExecutorEnv, ExecutorEnvBuilder};

use ethereum_rust_rlp::encode::RLPEncode;

use ethereum_rust_l2::proposer::prover_server::ProverInputData;
use ethereum_rust_l2::utils::config::prover_client::ProverClientConfig;

pub struct Prover<'a> {
    env_builder: ExecutorEnvBuilder<'a>,
    elf: &'a [u8],
    id: [u32; 8],
}

impl<'a> Default for Prover<'a> {
    fn default() -> Self {
        let config = ProverClientConfig::from_env().unwrap();
        Self::new_from_config(config)
    }
}

impl<'a> Prover<'a> {
    pub fn new_from_config(config: ProverClientConfig) -> Self {
        let _elf = std::fs::read(config.elf_path).unwrap();

        Self {
            env_builder: ExecutorEnv::builder(),
            elf: ZKVM_PROGRAM_ELF,
            id: ZKVM_PROGRAM_ID,
        }
    }

    pub fn set_input(&mut self, input: ProverInputData) -> &mut Self {
        let head_block_rlp = input.block.encode_to_vec();
        let parent_block_header_rlp = input.parent_block_header.encode_to_vec();

        // We should pass the inputs as a whole struct
        self.env_builder.write(&head_block_rlp).unwrap();
        self.env_builder.write(&parent_block_header_rlp).unwrap();
        self.env_builder.write(&input.db).unwrap();

        self
    }

    /// Example:
    /// let prover = Prover::new_from_config(prover_config);
    /// let proof = prover.set_input(inputs).prove().unwrap();
    pub fn prove(&mut self) -> Result<risc0_zkvm::Receipt, String> {
        let env = self
            .env_builder
            .build()
            .map_err(|_| "Failed to Build env".to_string())?;

        // Generate the Receipt
        let prover = default_prover();

        // Proof information by proving the specified ELF binary.
        // This struct contains the receipt along with statistics about execution of the guest
        let prove_info = prover
            .prove(env, self.elf)
            .map_err(|_| "Failed to prove".to_string())?;

        // extract the receipt.
        let receipt = prove_info.receipt;

        info!("Successfully generated Receipt!");
        Ok(receipt)
    }

    pub fn verify(&self, receipt: &risc0_zkvm::Receipt) -> Result<(), String> {
        // Verify the proof.
        receipt.verify(self.id).unwrap();
        Ok(())
    }
}
