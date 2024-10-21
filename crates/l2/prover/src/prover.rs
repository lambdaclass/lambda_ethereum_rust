use ethereum_rust_rlp::encode::RLPEncode;
use tracing::info;

use sp1_sdk::{ProverClient, SP1ProofWithPublicValues, SP1ProvingKey, SP1Stdin, SP1VerifyingKey};

use ethereum_rust_l2::proposer::prover_server::ProverInputData;
use ethereum_rust_l2::utils::config::prover_client::ProverClientConfig;

pub struct Prover {
    client: ProverClient,
    stdin: SP1Stdin,
    elf: Vec<u8>,
    pk: SP1ProvingKey,
    vk: SP1VerifyingKey,
}

impl Default for Prover {
    fn default() -> Self {
        let config = ProverClientConfig::from_env().unwrap();
        Self::new_from_config(config)
    }
}

impl Prover {
    pub fn new_from_config(config: ProverClientConfig) -> Self {
        let elf = std::fs::read(config.elf_path).unwrap();

        info!("Setting up prover...");
        let client = ProverClient::new();
        let (pk, vk) = client.setup(elf.as_slice());
        info!("Prover setup complete!");

        Self {
            client,
            stdin: SP1Stdin::new(),
            elf,
            pk,
            vk,
        }
    }

    pub fn set_input(&mut self, input: ProverInputData) -> &Self {
        self.stdin = SP1Stdin::new();
        let head_block_rlp = input.block.encode_to_vec();
        let parent_block_header_rlp = input.parent_block_header.encode_to_vec();

        // We should pass the inputs as a whole struct
        self.stdin.write(&head_block_rlp);
        self.stdin.write(&parent_block_header_rlp);
        self.stdin.write(&input.db);

        self
    }

    /// Example:
    /// let prover = Prover::new_from_config(prover_config);
    /// let proof = prover.set_input(inputs).execute().unwrap();
    pub fn execute(&self) -> Result<(), String> {
        let (mut raw_computed_public_inputs, report) = self
            .client
            .execute(&self.elf, self.stdin.clone())
            .run()
            .unwrap();
        println!("Program executed successfully.");

        let computed_public_inputs = raw_computed_public_inputs.read::<ProverInputData>();

        println!("Computed Public Inputs: {computed_public_inputs:#?}");
        println!("Instruction Count: {}", report.total_instruction_count());

        Ok(())
    }

    /// Example:
    /// let prover = Prover::new_from_config(prover_config);
    /// let proof = prover.set_input(inputs).prove().unwrap();
    pub fn prove(&self) -> Result<SP1ProofWithPublicValues, String> {
        // Generate the proof
        let proof = self
            .client
            .prove(&self.pk, self.stdin.clone())
            .plonk()
            .run()
            .map_err(|_| "Failed to generate proof".to_string())?;

        info!("Successfully generated proof!");
        Ok(proof)
    }

    pub fn verify(&self, proof: &SP1ProofWithPublicValues) -> Result<(), String> {
        // Verify the proof.
        self.client
            .verify(proof, &self.vk)
            .map_err(|_| "Failed to verify proof".to_string())?;
        info!("Successfully verified proof!");
        Ok(())
    }
}
