use ethrex_core::H256;
use ethrex_l2::proposer::prover_server::Risc0Proof;
use ethrex_l2::proposer::prover_server::Sp1Proof;
use tracing::info;
// risc0
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts};
use zkvm_interface::{
    io::{ProgramInput, ProgramOutput},
    methods::{ZKVM_PROGRAM_ELF, ZKVM_PROGRAM_ID},
};

// sp1
use sp1_sdk::{ProverClient, SP1Stdin};

use crate::errors::ProverError;

#[cfg(all(not(clippy), feature = "build_sp1"))]
pub const SP1_ELF: &[u8] = include_bytes!("../sp1/zkvm/elf/riscv32im-succinct-zkvm-elf");

#[cfg(any(clippy, not(feature = "build_sp1")))]
pub const SP1_ELF: &[u8] = &[0];

pub struct Risc0Prover<'a> {
    elf: &'a [u8],
    pub id: [u32; 8],
    pub stdout: Vec<u8>,
}

impl<'a> Default for Risc0Prover<'a> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Sp1Prover<'a> {
    elf: &'a [u8],
}

impl<'a> Default for Sp1Prover<'a> {
    fn default() -> Self {
        Self::new()
    }
}

// Boxing because of a clippy warning
pub enum ProvingOutput {
    Risc0Prover(Risc0Proof),
    Sp1Prover(Sp1Proof),
}

pub enum ProverType {
    RISC0,
    SP1,
}

pub fn create_prover(prover_type: ProverType) -> Box<dyn Prover> {
    match prover_type {
        ProverType::RISC0 => Box::new(Risc0Prover::new()),
        ProverType::SP1 => Box::new(Sp1Prover::new()),
    }
}

// Implement the Prover trait for the enum
pub trait Prover {
    fn prove(&mut self, input: ProgramInput) -> Result<ProvingOutput, Box<dyn std::error::Error>>;
    fn verify(&self, proving_output: &ProvingOutput) -> Result<(), Box<dyn std::error::Error>>;
    fn get_gas(&self) -> Result<u64, Box<dyn std::error::Error>>;
    fn get_commitment(
        &self,
        proving_output: &ProvingOutput,
    ) -> Result<ProgramOutput, Box<dyn std::error::Error>>;
}

impl<'a> Risc0Prover<'a> {
    pub fn new() -> Self {
        Self {
            elf: ZKVM_PROGRAM_ELF,
            id: ZKVM_PROGRAM_ID,
            stdout: Vec::new(),
        }
    }
}

impl<'a> Prover for Risc0Prover<'a> {
    fn prove(&mut self, input: ProgramInput) -> Result<ProvingOutput, Box<dyn std::error::Error>> {
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
        Ok(ProvingOutput::Risc0Prover(Risc0Proof::new(
            receipt,
            self.id.to_vec(),
        )))
    }

    fn verify(&self, proving_output: &ProvingOutput) -> Result<(), Box<dyn std::error::Error>> {
        // Verify the proof.
        match proving_output {
            ProvingOutput::Risc0Prover(proof) => proof.receipt.verify(self.id)?,
            ProvingOutput::Sp1Prover(_) => return Err(Box::new(ProverError::IncorrectProverType)),
        }

        Ok(())
    }

    fn get_gas(&self) -> Result<u64, Box<dyn std::error::Error>> {
        Ok(risc0_zkvm::serde::from_slice(
            self.stdout.get(..8).unwrap_or_default(), // first 8 bytes
        )?)
    }

    fn get_commitment(
        &self,
        proving_output: &ProvingOutput,
    ) -> Result<ProgramOutput, Box<dyn std::error::Error>> {
        let commitment = match proving_output {
            ProvingOutput::Risc0Prover(proof) => proof.receipt.journal.decode()?,
            ProvingOutput::Sp1Prover(_) => return Err(Box::new(ProverError::IncorrectProverType)),
        };
        Ok(commitment)
    }
}

impl<'a> Sp1Prover<'a> {
    pub fn new() -> Self {
        Self { elf: SP1_ELF }
    }
}

impl<'a> Prover for Sp1Prover<'a> {
    fn prove(&mut self, input: ProgramInput) -> Result<ProvingOutput, Box<dyn std::error::Error>> {
        let mut stdin = SP1Stdin::new();
        stdin.write(&input);

        // Generate the ProverClient
        let client = ProverClient::new();
        let (pk, vk) = client.setup(self.elf);

        // Proof information by proving the specified ELF binary.
        // This struct contains the receipt along with statistics about execution of the guest
        let proof = client.prove(&pk, stdin).groth16().run()?;
        // Wrap Proof and vk
        let sp1_proof = Sp1Proof::new(proof, vk);
        info!("Successfully generated SP1Proof.");
        Ok(ProvingOutput::Sp1Prover(sp1_proof))
    }

    fn verify(&self, proving_output: &ProvingOutput) -> Result<(), Box<dyn std::error::Error>> {
        // Verify the proof.
        match proving_output {
            ProvingOutput::Sp1Prover(complete_proof) => {
                let client = ProverClient::new();
                client.verify(&complete_proof.proof, &complete_proof.vk)?;
            }
            ProvingOutput::Risc0Prover(_) => {
                return Err(Box::new(ProverError::IncorrectProverType))
            }
        }

        Ok(())
    }

    fn get_gas(&self) -> Result<u64, Box<dyn std::error::Error>> {
        todo!()
    }

    fn get_commitment(
        &self,
        proving_output: &ProvingOutput,
    ) -> Result<ProgramOutput, Box<dyn std::error::Error>> {
        // TODO
        match proving_output {
            // TODO decode
            ProvingOutput::Sp1Prover(_complete_proof) => {
                //todo
            }
            ProvingOutput::Risc0Prover(_) => {
                return Err(Box::new(ProverError::IncorrectProverType))
            }
        }

        Ok(ProgramOutput {
            initial_state_hash: H256::zero(),
            final_state_hash: H256::zero(),
        })
    }
}
