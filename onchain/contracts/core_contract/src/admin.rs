use soroban_sdk::{panic_with_error, symbol_short, Address, BytesN, Env};

use crate::errors::CoreError;
use crate::events::{INIT_EVENT, ROLE_GRANTED};
use crate::{smt_root, storage};

pub struct Admin;

impl Admin {
    /// Initializes the contract with an owner. The owner is also set as the initial admin and operator.
    pub fn initialize(env: Env, owner: Address) {
        if storage::is_initialized(&env) {
            panic_with_error!(&env, CoreError::AlreadyInitialized);
        }
        owner.require_auth();
        storage::set_owner(&env, &owner);
        // By default, owner is also admin and operator
        storage::set_admin(&env, &owner);
        storage::set_operator(&env, &owner);
        #[allow(deprecated)]
        env.events().publish((INIT_EVENT,), (owner,));
    }

    /// Returns the current contract owner address.
    pub fn get_contract_owner(env: Env) -> Address {
        storage::get_owner(&env).unwrap_or_else(|| panic_with_error!(&env, CoreError::NotFound))
    }

    /// Returns the current admin address.
    pub fn get_admin(env: Env) -> Address {
        storage::get_admin(&env).unwrap_or_else(|| panic_with_error!(&env, CoreError::NotFound))
    }

    /// Returns the current operator address.
    pub fn get_operator(env: Env) -> Address {
        storage::get_operator(&env).unwrap_or_else(|| panic_with_error!(&env, CoreError::NotFound))
    }

    /// Sets a new admin address. Only the owner can call this.
    pub fn set_admin(env: Env, new_admin: Address) {
        let owner = Self::get_contract_owner(env.clone());
        owner.require_auth();
        storage::set_admin(&env, &new_admin);
        #[allow(deprecated)]
        env.events()
            .publish((ROLE_GRANTED, symbol_short!("admin")), (new_admin,));
    }

    /// Sets a new operator address. Only the admin can call this.
    pub fn set_operator(env: Env, new_operator: Address) {
        let admin = Self::get_admin(env.clone());
        admin.require_auth();
        storage::set_operator(&env, &new_operator);
        #[allow(deprecated)]
        env.events()
            .publish((ROLE_GRANTED, symbol_short!("operator")), (new_operator,));
    }

    pub fn get_smt_root(env: Env) -> BytesN<32> {
        smt_root::SmtRoot::get_root(env.clone())
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::RootNotSet))
    }

    /// Updates the Sparse Merkle Tree root. Only the operator can call this.
    pub fn update_smt_root(env: Env, new_root: BytesN<32>) {
        let operator = storage::get_operator(&env)
            .unwrap_or_else(|| panic_with_error!(&env, CoreError::NotFound));
        operator.require_auth();

        if let Some(current) = env
            .storage()
            .instance()
            .get::<_, soroban_sdk::BytesN<32>>(&storage::DataKey::SmtRoot)
        {
            if current == new_root {
                panic_with_error!(&env, CoreError::RootUnchanged);
            }
        }

        smt_root::SmtRoot::update_root(&env, new_root);
    }
}
