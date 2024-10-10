use serde::{Deserialize, Serialize};
use tracing::info;

#[allow(unused_imports)]
use prover_lib::inputs::{ProverInput, ProverInputNoExecution};
use sp1_sdk::{ProverClient, SP1ProofWithPublicValues, SP1ProvingKey, SP1Stdin, SP1VerifyingKey};

use ethereum_rust_rlp::encode::RLPEncode;

use crate::utils::config::prover::ProverConfig;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const VERIFICATION_ELF: &[u8] =
    include_bytes!("./sp1/verification_program/elf/riscv32im-succinct-zkvm-elf");
pub const EXECUTION_ELF: &[u8] =
    include_bytes!("./sp1/execution_program/elf/riscv32im-succinct-zkvm-elf");

pub struct Prover {
    client: ProverClient,
    pk: SP1ProvingKey,
    vk: SP1VerifyingKey,
    pub mode: ProverMode,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum ProverMode {
    Verification,
    Execution,
}

impl Prover {
    pub fn new_from_config(config: ProverConfig) -> Self {
        let elf = std::fs::read(config.elf_path).unwrap();

        info!("Setting up prover...");
        let client = ProverClient::new();
        let (pk, vk) = client.setup(elf.as_slice());
        info!("Prover setup complete!");

        // TODO set prover mode in config? or add a way to handle multiple elfs
        Self {
            client,
            pk,
            vk,
            mode: ProverMode::Execution,
        }
    }

    // Not used atm
    pub fn prove_verification(
        &self,
        input: &ProverInputNoExecution,
    ) -> Result<SP1ProofWithPublicValues, String> {
        let head_block_rlp = input.head_block.encode_to_vec();
        let parent_block_header_rlp = input.parent_block_header.encode_to_vec();

        // Setup the inputs.
        let mut stdin = SP1Stdin::new();

        stdin.write(&head_block_rlp);
        stdin.write(&parent_block_header_rlp);

        info!(
            "Starting block execution proof for block = {:?}",
            input.head_block
        );

        // Generate the proof
        let proof = self
            .client
            .prove(&self.pk, stdin)
            .groth16()
            .run()
            .map_err(|_| "Failed to generate proof".to_string())?;

        info!("Successfully generated proof!");

        // Verify the proof.
        self.client
            .verify(&proof, &self.vk)
            .map_err(|_| "Failed to verify proof".to_string())?;
        info!("Successfully verified proof!");

        Ok(proof)
    }

    pub fn prove_execution(&self, input: &ProverInput) -> Result<SP1ProofWithPublicValues, String> {
        let head_block_rlp = input.block.clone().encode_to_vec();
        let parent_header_rlp = input.parent_block_header.clone().encode_to_vec();

        // Write the inputs
        let mut stdin = SP1Stdin::new();
        stdin.write(&head_block_rlp);
        stdin.write(&parent_header_rlp);
        stdin.write(&input.db);

        info!(
            "Starting block execution proof for block = {:?}",
            input.block
        );

        // Generate the proof
        let proof = self
            .client
            .prove(&self.pk, stdin)
            .groth16()
            .run()
            .map_err(|_| "Failed to generate proof".to_string())?;

        info!("Successfully generated proof!");

        // Verify the proof.
        self.client
            .verify(&proof, &self.vk)
            .map_err(|_| "Failed to verify proof".to_string())?;
        info!("Successfully verified proof!");

        Ok(proof)
    }
}
