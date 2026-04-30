use soroban_sdk::{panic_with_error, Address, Bytes, BytesN, Env};

use crate::errors::CoreError;
use crate::events::TRANSFER_EVENT;
use crate::registration;
use crate::types::PublicSignals;
use crate::{smt_root, zk_verifier};

pub struct Transfer;

impl Transfer {
    pub fn transfer_ownership(
        env: Env,
        caller: Address,
        commitment: BytesN<32>,
        new_owner: Address,
    ) {
        caller.require_auth();
        let key = registration::DataKey::Commitment(commitment.clone());
        let current_owner: Address = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::NotFound));
        if caller != current_owner {
            panic_with_error!(&env, CoreError::Unauthorized);
        }
        if new_owner == current_owner {
            panic_with_error!(&env, CoreError::SameOwner);
        }
        env.storage().persistent().set(&key, &new_owner);
        #[allow(deprecated)]
        env.events()
            .publish((TRANSFER_EVENT,), (commitment, caller, new_owner));
    }

    pub fn transfer(
        env: Env,
        caller: Address,
        commitment: BytesN<32>,
        new_owner: Address,
        proof: Bytes,
        public_signals: PublicSignals,
    ) {
        caller.require_auth();
        let key = registration::DataKey::Commitment(commitment.clone());
        let current_owner: Address = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::NotFound));
        if caller != current_owner {
            panic_with_error!(&env, CoreError::Unauthorized);
        }
        if new_owner == current_owner {
            panic_with_error!(&env, CoreError::SameOwner);
        }
        let current_root = smt_root::SmtRoot::get_root(env.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::RootNotSet));
        if public_signals.old_root != current_root {
            panic_with_error!(&env, CoreError::StaleRoot);
        }
        if public_signals.commitment != commitment {
            panic_with_error!(&env, CoreError::InvalidProof);
        }
        if !zk_verifier::ZkVerifier::verify_groth16_proof(&env, &proof, &public_signals) {
            panic_with_error!(&env, CoreError::InvalidProof);
        }
        env.storage().persistent().set(&key, &new_owner);
        smt_root::SmtRoot::update_root(&env, public_signals.new_root);
        #[allow(deprecated)]
        env.events()
            .publish((TRANSFER_EVENT,), (commitment, caller, new_owner));
    }
}
