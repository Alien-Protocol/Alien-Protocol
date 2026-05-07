use soroban_sdk::{Address, Bytes, BytesN, Env};

use crate::errors::CoreError;
use crate::events::{emit_privacy_set, emit_register};
use crate::registration::Registration;
use crate::storage;
use crate::types::{Permission, PrivacyMode, PublicSignals, ResolveData};
use crate::{smt_root, zk_verifier};

pub struct Resolver;

impl Resolver {
    pub fn register_resolver(
        e: Env,
        caller: Address,
        commitment: BytesN<32>,
        proof: Bytes,
        public_signals: PublicSignals,
    ) -> Result<(), CoreError> {
        caller.require_auth();

        let key = storage::DataKey::Resolver(commitment.clone());
        if e.storage().persistent().has(&key) {
            return Err(CoreError::DuplicateCommitment);
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

        let data = ResolveData {
            wallet: caller.clone(),
            memo: None,
        };
        e.storage().persistent().set(&key, &data);

        smt_root::SmtRoot::update_root(&e, public_signals.new_root);

        emit_register(&e, &commitment, &caller);
        Ok(())
    }

    pub fn set_memo(e: Env, caller: Address, commitment: BytesN<32>, memo_id: u64) -> Result<(), CoreError> {
        caller.require_auth();

        let mut data = e
            .storage()
            .persistent()
            .get::<storage::DataKey, ResolveData>(&storage::DataKey::Resolver(commitment.clone()))
            .ok_or(CoreError::NotFound)?;

        let owner = Registration::get_owner(e.clone(), commitment.clone()).ok_or(CoreError::NotFound)?;

        if owner != caller
            && !storage::has_permission(&e, &commitment, &caller, Permission::SetMemo)
        {
            return Err(CoreError::Unauthorized);
        }

        data.memo = Some(memo_id);
        e.storage()
            .persistent()
            .set(&storage::DataKey::Resolver(commitment), &data);
        Ok(())
    }

    pub fn set_privacy_mode(
        e: Env,
        caller: Address,
        username_hash: BytesN<32>,
        mode: PrivacyMode,
    ) -> Result<(), CoreError> {
        caller.require_auth();

        let owner = Registration::get_owner(e.clone(), username_hash.clone()).ok_or(CoreError::NotFound)?;

        if owner != caller
            && !storage::has_permission(&e, &username_hash, &caller, Permission::SetPrivacyMode)
        {
            return Err(CoreError::Unauthorized);
        }

        storage::set_privacy_mode(&e, &username_hash, &mode);

        emit_privacy_set(&e, &username_hash, &mode);
        Ok(())
    }

    pub fn get_privacy_mode(e: Env, username_hash: BytesN<32>) -> PrivacyMode {
        storage::get_privacy_mode(&e, &username_hash)
    }

    pub fn resolve(e: Env, commitment: BytesN<32>) -> Result<(Address, Option<u64>), CoreError> {
        let data = e
            .storage()
            .persistent()
            .get::<storage::DataKey, ResolveData>(&storage::DataKey::Resolver(commitment.clone()))
            .ok_or(CoreError::NotFound)?;

        if storage::get_privacy_mode(&e, &commitment) == PrivacyMode::Shielded {
            Ok((e.current_contract_address(), data.memo))
        } else {
            Ok((data.wallet, data.memo))
        }
    }
}
