use serde::{Deserialize, Serialize};
use tracing::info;

#[allow(unused_imports)]
use prover_lib::inputs::{ProverInput, ProverInputNoExecution};
use sp1_sdk::{ProverClient, SP1ProofWithPublicValues, SP1ProvingKey, SP1Stdin, SP1VerifyingKey};

use ethereum_rust_rlp::encode::RLPEncode;

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

impl Default for Prover {
    fn default() -> Self {
        Self::new_verification()
    }
}

impl Prover {
    pub fn new_verification() -> Self {
        info!("Setting up Verification Prover...");
        let client = ProverClient::new();
        let (pk, vk) = client.setup(VERIFICATION_ELF);
        info!("Verification Prover setup complete!");

        Self {
            client,
            pk,
            vk,
            mode: ProverMode::Verification,
        }
    }

    pub fn new_execution() -> Self {
        info!("Setting up Verification Prover...");
        let client = ProverClient::new();
        let (pk, vk) = client.setup(VERIFICATION_ELF);
        info!("Verification Prover setup complete!");

        Self {
            client,
            pk,
            vk,
            mode: ProverMode::Execution,
        }
    }

    pub fn prove(&self, input: Box<dyn std::any::Any>) -> Result<SP1ProofWithPublicValues, String> {
        match self.mode {
            ProverMode::Verification => {
                if let Some(verification_input) = input.downcast_ref::<ProverInputNoExecution>() {
                    self.prove_verification(verification_input)
                } else {
                    Err("Invalid input type for Verification".to_string())
                }
            }
            ProverMode::Execution => {
                if let Some(execution_input) = input.downcast_ref::<ProverInput>() {
                    self.prove_execution(execution_input)
                } else {
                    Err("Invalid input type for Execution".to_string())
                }
            }
        }
    }

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
        //let memory_db = input.db;

        // Setup the inputs.
        let mut stdin = SP1Stdin::new();

        stdin.write(&head_block_rlp);
        // TODO write db

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
