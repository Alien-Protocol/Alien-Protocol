use soroban_sdk::{panic_with_error, Address, Bytes, BytesN, Env};

use crate::errors::CoreError;
use crate::events::{privacy_set_event, REGISTER_EVENT};
use crate::registration::Registration;
use crate::storage;
use crate::types::{Permission, PrivacyMode, PublicSignals, ResolveData};
use crate::{smt_root, zk_verifier};

pub struct Resolver;

impl Resolver {
    pub fn register_resolver(
        env: Env,
        caller: Address,
        commitment: BytesN<32>,
        proof: Bytes,
        public_signals: PublicSignals,
    ) {
        caller.require_auth();

        let key = storage::DataKey::Resolver(commitment.clone());
        if env.storage().persistent().has(&key) {
            panic_with_error!(&env, CoreError::DuplicateCommitment);
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

        let data = ResolveData {
            wallet: caller.clone(),
            memo: None,
        };
        env.storage().persistent().set(&key, &data);

        smt_root::SmtRoot::update_root(&env, public_signals.new_root);

        #[allow(deprecated)]
        env.events()
            .publish((REGISTER_EVENT,), (commitment, caller));
    }

    pub fn set_memo(env: Env, caller: Address, commitment: BytesN<32>, memo_id: u64) {
        caller.require_auth();

        let mut data = env
            .storage()
            .persistent()
            .get::<storage::DataKey, ResolveData>(&storage::DataKey::Resolver(commitment.clone()))
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::NotFound));

        let owner = Registration::get_owner(env.clone(), commitment.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::NotFound));

        if owner != caller
            && !storage::has_permission(&env, &commitment, &caller, Permission::SetMemo)
        {
            panic_with_error!(&env, CoreError::Unauthorized);
        }

        data.memo = Some(memo_id);
        env.storage()
            .persistent()
            .set(&storage::DataKey::Resolver(commitment), &data);
    }

    pub fn set_privacy_mode(
        env: Env,
        caller: Address,
        username_hash: BytesN<32>,
        mode: PrivacyMode,
    ) {
        caller.require_auth();

        let owner = Registration::get_owner(env.clone(), username_hash.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::NotFound));

        if owner != caller
            && !storage::has_permission(&env, &username_hash, &caller, Permission::SetPrivacyMode)
        {
            panic_with_error!(&env, CoreError::Unauthorized);
        }

        storage::set_privacy_mode(&env, &username_hash, &mode);

        #[allow(deprecated)]
        env.events()
            .publish((privacy_set_event(&env),), (username_hash, mode));
    }

    pub fn get_privacy_mode(env: Env, username_hash: BytesN<32>) -> PrivacyMode {
        storage::get_privacy_mode(&env, &username_hash)
    }

    pub fn resolve(env: Env, commitment: BytesN<32>) -> (Address, Option<u64>) {
        match env
            .storage()
            .persistent()
            .get::<storage::DataKey, ResolveData>(&storage::DataKey::Resolver(commitment.clone()))
        {
            Some(data) => {
                if storage::get_privacy_mode(&env, &commitment) == PrivacyMode::Shielded {
                    (env.current_contract_address(), data.memo)
                } else {
                    (data.wallet, data.memo)
                }
            }
            None => panic_with_error!(&env, CoreError::NotFound),
        }
    }
}
