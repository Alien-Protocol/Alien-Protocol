use soroban_sdk::{Address, Bytes, BytesN, Env};

use crate::errors::CoreError;
use crate::events::emit_transfer;
use crate::registration;
use crate::types::PublicSignals;
use crate::{smt_root, zk_verifier};

pub struct Transfer;

impl Transfer {
    pub fn transfer_ownership(
        e: Env,
        caller: Address,
        commitment: BytesN<32>,
        new_owner: Address,
    ) -> Result<(), CoreError> {
        caller.require_auth();
        let key = registration::DataKey::Commitment(commitment.clone());
        let current_owner: Address = e
            .storage()
            .persistent()
            .get(&key)
            .ok_or(CoreError::NotFound)?;
        if caller != current_owner {
            return Err(CoreError::Unauthorized);
        }
        if new_owner == current_owner {
            return Err(CoreError::SameOwner);
        }
        e.storage().persistent().set(&key, &new_owner);
        emit_transfer(&e, &commitment, &current_owner, &new_owner);
        Ok(())
    }

    pub fn transfer(
        e: Env,
        caller: Address,
        commitment: BytesN<32>,
        new_owner: Address,
        proof: Bytes,
        public_signals: PublicSignals,
    ) -> Result<(), CoreError> {
        caller.require_auth();
        let key = registration::DataKey::Commitment(commitment.clone());
        let current_owner: Address = e
            .storage()
            .persistent()
            .get(&key)
            .ok_or(CoreError::NotFound)?;
        if caller != current_owner {
            return Err(CoreError::Unauthorized);
        }
        if new_owner == current_owner {
            return Err(CoreError::SameOwner);
        }
        let current_root = smt_root::SmtRoot::get_root(e.clone()).ok_or(CoreError::RootNotSet)?;
        if public_signals.old_root != current_root {
            return Err(CoreError::StaleRoot);
        }
        if public_signals.commitment != commitment {
            return Err(CoreError::InvalidProof);
        }
        if !zk_verifier::ZkVerifier::verify_groth16_proof(&e, &proof, &public_signals) {
            return Err(CoreError::InvalidProof);
        }
        e.storage().persistent().set(&key, &new_owner);
        smt_root::SmtRoot::update_root(&e, public_signals.new_root);
        emit_transfer(&e, &commitment, &current_owner, &new_owner);
        Ok(())
    }
}
