use shared::auth as shared_auth;
use soroban_sdk::{symbol_short, Address, BytesN, Env};

use crate::errors::CoreError;
use crate::events::{emit_init, emit_role_granted, emit_root_updated};
use crate::{smt_root, storage};

pub struct Admin;

impl Admin {
    pub fn initialize(e: Env, owner: Address) -> Result<(), CoreError> {
        if storage::is_initialized(&e) {
            return Err(CoreError::AlreadyInitialized);
        }
        owner.require_auth();
        storage::set_owner(&e, &owner);
        storage::set_admin(&e, &owner);
        storage::set_operator(&e, &owner);
        emit_init(&e, &owner);
        Ok(())
    }

    pub fn get_contract_owner(e: Env) -> Address {
        shared_auth::unwrap_or_panic(&e, storage::get_owner(&e), CoreError::NotFound)
    }

    pub fn get_admin(e: Env) -> Address {
        shared_auth::unwrap_or_panic(&e, storage::get_admin(&e), CoreError::NotFound)
    }

    pub fn get_operator(e: Env) -> Address {
        shared_auth::unwrap_or_panic(&e, storage::get_operator(&e), CoreError::NotFound)
    }

    pub fn set_admin(e: Env, new_admin: Address) -> Result<(), CoreError> {
        let owner = storage::get_owner(&e).ok_or(CoreError::NotFound)?;
        owner.require_auth();
        storage::set_admin(&e, &new_admin);
        emit_role_granted(&e, &symbol_short!("admin"), &new_admin);
        Ok(())
    }

    pub fn set_operator(e: Env, new_operator: Address) -> Result<(), CoreError> {
        let admin = storage::get_admin(&e).ok_or(CoreError::NotFound)?;
        admin.require_auth();
        storage::set_operator(&e, &new_operator);
        emit_role_granted(&e, &symbol_short!("operator"), &new_operator);
        Ok(())
    }

    pub fn get_smt_root(e: Env) -> BytesN<32> {
        shared_auth::unwrap_or_panic(&e, smt_root::SmtRoot::get_root(e.clone()), CoreError::RootNotSet)
    }

    pub fn update_smt_root(e: Env, new_root: BytesN<32>) -> Result<(), CoreError> {
        let operator = storage::get_operator(&e).ok_or(CoreError::NotFound)?;
        operator.require_auth();

        let old_root = e
            .storage()
            .instance()
            .get::<_, soroban_sdk::BytesN<32>>(&storage::DataKey::SmtRoot);

        if let Some(current) = old_root.clone() {
            if current == new_root {
                return Err(CoreError::RootUnchanged);
            }
        }

        smt_root::SmtRoot::update_root(&e, new_root.clone());
        emit_root_updated(&e, old_root.as_ref(), &new_root);
        Ok(())
    }
}
