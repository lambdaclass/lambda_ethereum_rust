use tracing::info;

use sp1_sdk::{ProverClient, SP1ProofWithPublicValues, SP1ProvingKey, SP1Stdin, SP1VerifyingKey};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const FIBONACCI_ELF: &[u8] = include_bytes!("./sp1/program/elf/riscv32im-succinct-zkvm-elf");

pub struct SP1Prover {
    client: ProverClient,
    pk: SP1ProvingKey,
    vk: SP1VerifyingKey,
}

impl Default for SP1Prover {
    fn default() -> Self {
        Self::new()
    }
}

impl SP1Prover {
    pub fn new() -> Self {
        info!("Setting up prover...");
        let client = ProverClient::new();
        let (pk, vk) = client.setup(FIBONACCI_ELF);
        info!("Prover setup complete!");

        Self { client, pk, vk }
    }

    pub fn prove(&self, id: u32) -> Result<SP1ProofWithPublicValues, String> {
        // Setup the inputs.
        let mut stdin = SP1Stdin::new();
        stdin.write(&id);

        info!("Starting Fibonacci proof for n = {}", id);

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
