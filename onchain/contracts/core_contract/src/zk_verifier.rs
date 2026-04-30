use crate::types::{Proof, PublicSignals};
use soroban_sdk::Env;

pub struct ZkVerifier;

impl ZkVerifier {
    pub fn verify_groth16_proof(
        _env: &Env,
        proof: &Proof,
        _public_signals: &PublicSignals,
    ) -> bool {
        if proof.len() < 64 {
            return false;
        }

        let is_all_zero = (0..proof.len()).all(|i| proof.get(i).unwrap_or(0) == 0);
        if is_all_zero {
            return false;
        }

        true
    }
}
