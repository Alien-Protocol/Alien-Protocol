use crate::errors::CoreError;
use crate::events::{emit_delegate_granted, emit_delegate_revoked, emit_register, emit_username_reg};
use crate::storage::{self, PERSISTENT_BUMP_AMOUNT, PERSISTENT_LIFETIME_THRESHOLD};
use crate::types::{PermissionSet, Proof, PublicSignals};
use crate::{smt_root, zk_verifier};
use soroban_sdk::{contracttype, Address, BytesN, Env};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Commitment(BytesN<32>),
}

pub struct Registration;

impl Registration {
    pub fn submit_proof(e: Env, caller: Address, proof: Proof, public_signals: PublicSignals) -> Result<(), CoreError> {
        caller.require_auth();

        let commitment = public_signals.commitment.clone();
        let key = DataKey::Commitment(commitment.clone());
        if e.storage().persistent().has(&key) {
            return Err(CoreError::AlreadyRegistered);
        }

        let current_root = smt_root::SmtRoot::get_root(e.clone()).ok_or(CoreError::RootNotSet)?;
        if public_signals.old_root != current_root {
            return Err(CoreError::StaleRoot);
        }

        if !zk_verifier::ZkVerifier::verify_groth16_proof(&e, &proof, &public_signals) {
            return Err(CoreError::InvalidProof);
        }

        e.storage().persistent().set(&key, &caller);
        e.storage().persistent().extend_ttl(
            &key,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );

        storage::set_created_at(&e, &commitment, e.ledger().timestamp());
        smt_root::SmtRoot::update_root(&e, public_signals.new_root);

        emit_username_reg(&e, &commitment);
        Ok(())
    }

    pub fn register(e: Env, caller: Address, commitment: BytesN<32>) -> Result<(), CoreError> {
        caller.require_auth();

        let key = DataKey::Commitment(commitment.clone());
        if e.storage().persistent().has(&key) {
            return Err(CoreError::AlreadyRegistered);
        }

        e.storage().persistent().set(&key, &caller);
        e.storage().persistent().extend_ttl(
            &key,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );

        storage::set_created_at(&e, &commitment, e.ledger().timestamp());

        emit_register(&e, &commitment, &caller);
        Ok(())
    }

    pub fn get_owner(e: Env, commitment: BytesN<32>) -> Option<Address> {
        let key = DataKey::Commitment(commitment);
        e.storage().persistent().get(&key)
    }

    pub fn get_created_at(e: Env, commitment: BytesN<32>) -> Option<u64> {
        storage::get_created_at(&e, &commitment)
    }

    pub fn grant_delegate(
        e: Env,
        owner: Address,
        username_hash: BytesN<32>,
        delegate: Address,
        permissions: PermissionSet,
    ) -> Result<(), CoreError> {
        owner.require_auth();

        let real_owner = Self::get_owner(e.clone(), username_hash.clone()).ok_or(CoreError::NotFound)?;

        if real_owner != owner {
            return Err(CoreError::Unauthorized);
        }

        storage::set_delegate_permissions(&e, &username_hash, &delegate, &permissions);

        emit_delegate_granted(&e, &username_hash, &delegate, permissions.permissions.len() as u32);
        Ok(())
    }

    pub fn revoke_delegate(e: Env, owner: Address, username_hash: BytesN<32>, delegate: Address) -> Result<(), CoreError> {
        owner.require_auth();

        let real_owner = Self::get_owner(e.clone(), username_hash.clone()).ok_or(CoreError::NotFound)?;

        if real_owner != owner {
            return Err(CoreError::Unauthorized);
        }

        storage::remove_delegate_permissions(&e, &username_hash, &delegate);

        emit_delegate_revoked(&e, &username_hash, &delegate);
        Ok(())
    }
}
