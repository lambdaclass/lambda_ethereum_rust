use crate::proposer::errors::ProverServerError;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use risc0_zkvm::sha::Digestible;
use sp1_sdk::HashableKey;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
/// Enum used to identify the different proving systems.
pub enum ProverType {
    RISC0,
    SP1,
}

/// Used to iterate through all the possible proving systems
impl ProverType {
    pub fn all() -> &'static [ProverType] {
        &[ProverType::RISC0, ProverType::SP1]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Risc0Proof {
    pub receipt: Box<risc0_zkvm::Receipt>,
    pub prover_id: Vec<u32>,
}

pub struct Risc0ContractData {
    pub block_proof: Vec<u8>,
    pub image_id: Vec<u8>,
    pub journal_digest: Vec<u8>,
}

impl Risc0Proof {
    pub fn new(receipt: risc0_zkvm::Receipt, prover_id: Vec<u32>) -> Self {
        Risc0Proof {
            receipt: Box::new(receipt),
            prover_id,
        }
    }

    pub fn contract_data(&self) -> Result<Risc0ContractData, ProverServerError> {
        // If we run the prover_client with RISC0_DEV_MODE=0 we will have a groth16 proof
        // Else, we will have a fake proof.
        //
        // The RISC0_DEV_MODE=1 should only be used with DEPLOYER_CONTRACT_VERIFIER=0xAA
        let block_proof = match self.receipt.inner.groth16() {
            Ok(inner) => {
                // The SELECTOR is used to perform an extra check inside the groth16 verifier contract.
                let mut selector =
                    hex::encode(inner.verifier_parameters.as_bytes().get(..4).ok_or(
                        ProverServerError::Custom(
                            "Failed to get verify_proof_selector in send_proof()".to_owned(),
                        ),
                    )?);
                let seal = hex::encode(inner.clone().seal);
                selector.push_str(&seal);
                hex::decode(selector).map_err(|e| {
                    ProverServerError::Custom(format!("Failed to hex::decode(selector): {e}"))
                })?
            }
            Err(_) => vec![32; 0],
        };

        let mut image_id: [u32; 8] = [0; 8];
        for (i, b) in image_id.iter_mut().enumerate() {
            *b = *self.prover_id.get(i).ok_or(ProverServerError::Custom(
                "Failed to get image_id in handle_proof_submission()".to_owned(),
            ))?;
        }

        let image_id: risc0_zkvm::sha::Digest = image_id.into();
        let image_id = image_id.as_bytes().to_vec();

        let journal_digest = Digestible::digest(&self.receipt.journal)
            .as_bytes()
            .to_vec();

        Ok(Risc0ContractData {
            block_proof,
            image_id,
            journal_digest,
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Sp1Proof {
    pub proof: Box<sp1_sdk::SP1ProofWithPublicValues>,
    pub vk: sp1_sdk::SP1VerifyingKey,
}

impl Debug for Sp1Proof {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sp1Proof")
            .field("proof", &self.proof)
            .field("vk", &self.vk.bytes32())
            .finish()
    }
}

pub struct Sp1ContractData {
    pub public_values: Vec<u8>,
    pub vk: Vec<u8>,
    pub proof_bytes: Vec<u8>,
}

impl Sp1Proof {
    pub fn new(
        proof: sp1_sdk::SP1ProofWithPublicValues,
        verifying_key: sp1_sdk::SP1VerifyingKey,
    ) -> Self {
        Sp1Proof {
            proof: Box::new(proof),
            vk: verifying_key,
        }
    }

    pub fn contract_data(&self) -> Result<Sp1ContractData, ProverServerError> {
        let vk = self
            .vk
            .bytes32()
            .strip_prefix("0x")
            .ok_or(ProverServerError::Custom(
                "Failed to strip_prefix of sp1 vk".to_owned(),
            ))?
            .to_string();
        let vk_bytes = hex::decode(&vk)
            .map_err(|_| ProverServerError::Custom("Failed hex::decode(&vk)".to_owned()))?;

        Ok(Sp1ContractData {
            public_values: self.proof.public_values.to_vec(),
            vk: vk_bytes,
            proof_bytes: self.proof.bytes(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ProvingOutput {
    RISC0(Risc0Proof),
    SP1(Sp1Proof),
}
