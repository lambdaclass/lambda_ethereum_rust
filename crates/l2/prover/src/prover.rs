use tracing::info;

// risc0
pub use zkvm_interface::io::{ProgramInput, ProgramOutput};
use zkvm_interface::methods::{ZKVM_PROGRAM_ELF, ZKVM_PROGRAM_ID};

use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts};

pub struct Prover<'a> {
    elf: &'a [u8],
    pub id: [u32; 8],
    pub stdout: Vec<u8>,
}

impl<'a> Default for Prover<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Prover<'a> {
    pub fn new() -> Self {
        Self {
            elf: ZKVM_PROGRAM_ELF,
            id: ZKVM_PROGRAM_ID,
            stdout: Vec::new(),
        }
    }

    pub fn prove(
        &mut self,
        input: ProgramInput,
    ) -> Result<risc0_zkvm::Receipt, Box<dyn std::error::Error>> {
        let env = ExecutorEnv::builder()
            .stdout(&mut self.stdout)
            .write(&input)?
            .build()?;

        // Generate the Receipt
        let prover = default_prover();

        // Proof information by proving the specified ELF binary.
        // This struct contains the receipt along with statistics about execution of the guest
        let prove_info = prover.prove_with_opts(env, self.elf, &ProverOpts::groth16())?;

        // Extract the receipt.
        let receipt = prove_info.receipt;

        info!("Successfully generated execution receipt.");
        Ok(receipt)
    }

    pub fn verify(&self, receipt: &risc0_zkvm::Receipt) -> Result<(), Box<dyn std::error::Error>> {
        // Verify the proof.
        receipt.verify(self.id)?;
        Ok(())
    }

    pub fn get_gas(&self) -> Result<u64, Box<dyn std::error::Error>> {
        Ok(risc0_zkvm::serde::from_slice(
            self.stdout.get(..8).unwrap_or_default(), // first 8 bytes
        )?)
    }

    pub fn get_commitment(
        receipt: &risc0_zkvm::Receipt,
    ) -> Result<ProgramOutput, Box<dyn std::error::Error>> {
        Ok(receipt.journal.decode()?)
    }
}
