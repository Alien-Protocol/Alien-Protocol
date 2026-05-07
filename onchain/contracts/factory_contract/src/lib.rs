#![no_std]

mod errors;
mod events;
mod storage;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, symbol_short, Address, BytesN, Env};

use crate::errors::FactoryError;
use crate::events::{emit_ownership_transferred, emit_username_deployed, ROLE_GRANTED};
use crate::storage::{
    get_admin, get_core_contract as read_core_contract, get_core_wasm_hash, get_operator,
    get_owner, get_username, set_admin, set_core_contract, set_core_wasm_hash, set_operator,
    set_owner, set_username,
};
use crate::types::UsernameRecord;

#[contract]
pub struct FactoryContract;

#[contractimpl]
impl FactoryContract {
    pub fn initialize(
        env: Env,
        owner: Address,
        admin: Address,
        oprator: Address,
        core_wasm_hash: BytesN<32>,
    ) -> Result<(), FactoryError> {
        if get_owner(&env).is_some() {
            return Err(FactoryError::Unauthorized);
        }
        owner.require_auth();
        set_owner(&env, &owner);
        set_admin(&env, &admin);
        set_operator(&env, &oprator);
        set_core_wasm_hash(&env, &core_wasm_hash);
        Ok(())
    }

    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), FactoryError> {
        let owner = get_owner(&env).ok_or(FactoryError::NotInitilizedContract)?;
        owner.require_auth();
        set_admin(&env, &new_admin);
        #[allow(deprecated)]
        env.events()
            .publish((ROLE_GRANTED, symbol_short!("admin")), (new_admin,));
        Ok(())
    }

    pub fn set_operator(env: Env, new_operator: Address) -> Result<(), FactoryError> {
        let admin = get_admin(&env).ok_or(FactoryError::NotInitilizedContract)?;
        admin.require_auth();
        set_operator(&env, &new_operator);
        #[allow(deprecated)]
        env.events()
            .publish((ROLE_GRANTED, symbol_short!("operator")), (new_operator,));
        Ok(())
    }

    pub fn transfer_username(
        env: Env,
        username_hash: BytesN<32>,
        new_owner: Address,
    ) -> Result<(), FactoryError> {
        let mut record = get_username(&env, &username_hash).ok_or(FactoryError::Unauthorized)?;
        let old_owner = record.owner.clone();
        record.owner = new_owner.clone();
        set_username(&env, &username_hash, &record);
        emit_ownership_transferred(&env, &username_hash, &old_owner, &new_owner);
        Ok(())
    }

    pub fn get_owner(env: Env) -> Option<Address> {
        get_owner(&env)
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        get_admin(&env)
    }

    pub fn get_operator(env: Env) -> Option<Address> {
        get_operator(&env)
    }

    pub fn get_username_record(env: Env, username_hash: BytesN<32>) -> Option<UsernameRecord> {
        get_username(&env, &username_hash)
    }

    pub fn deploy_core(
        e: Env,
        username_hash: BytesN<32>,
        resolver: Address,
        salt: BytesN<32>,
    ) -> Result<(), FactoryError> {
        resolver.require_auth();

        if username_hash.is_empty() {
            return Err(FactoryError::InvalidUsername);
        }

        let wasm_hash = match get_core_wasm_hash(&e) {
            Some(x) => x,
            None => return Err(FactoryError::NotInitilizedContract),
        };

        let constructor_args = (resolver.clone(), username_hash.clone());

        let deployed_address = e
            .deployer()
            .with_address(e.current_contract_address(), salt)
            .deploy_v2(wasm_hash, constructor_args);

        let record = UsernameRecord {
            username_hash: username_hash.clone(),
            owner: resolver,
            registered_at: e.ledger().timestamp(),
            core_contract: deployed_address.clone(),
        };

        set_core_contract(&e, username_hash.clone(), &deployed_address);
        set_username(&e, &username_hash, &record);
        emit_username_deployed(&e, &username_hash, &record.owner, record.registered_at);

        Ok(())
    }

    pub fn get_username_owner(env: Env, username_hash: BytesN<32>) -> Option<Address> {
        get_username(&env, &username_hash).map(|r| r.owner)
    }

    pub fn core_contract(env: Env, username_hash: BytesN<32>) -> Option<Address> {
        read_core_contract(&env, username_hash)
    }
}
