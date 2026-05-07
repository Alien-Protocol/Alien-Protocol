#![no_std]

mod errors;
mod events;
mod storage;
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, panic_with_error, symbol_short, Address, BytesN, Env};

use crate::errors::FactoryError;
use crate::events::{emit_ownership_transferred, emit_username_deployed, ROLE_GRANTED};
use crate::storage::{
    get_admin, get_auction_contract as read_auction_contract,
    get_core_contract as read_core_contract, get_operator, get_owner, get_username, has_username,
    set_admin, set_operator, set_owner, set_username,
};
use crate::types::UsernameRecord;

#[contract]
pub struct FactoryContract;

#[contractimpl]
impl FactoryContract {
    pub fn initialize(env: Env, owner: Address) {
        if get_owner(&env).is_some() {
            return Err(FactoryError::Unauthorized);
        }
        owner.require_auth();
        set_owner(&env, &owner);
        set_admin(&env, &admin);
        set_operator(&env, &oprator);

        Ok(())
    }

    pub fn configure(env: Env, auction_contract: Address, core_contract: Address) {
        let operator = get_operator(&env)
            .unwrap_or_else(|| panic_with_error!(&env, FactoryError::Unauthorized));
        operator.require_auth();
        set_auction_contract(&env, &auction_contract);
        set_core_contract(&env, &core_contract);
    }

    pub fn set_admin(env: Env, new_admin: Address) {
        let owner =
            match get_owner(&env) {
                Some(x) => x , 
                None => return  Err(FactoryError::NotInitilizedContract)
            };

        owner.require_auth();

        set_admin(&env, &new_admin);
        
        #[allow(deprecated)]
        env.events()
            .publish((ROLE_GRANTED, symbol_short!("admin")), (new_admin,));

        Ok(())
    }

    pub fn set_operator(env: Env, new_operator: Address) {
        let admin =
            get_admin(&env).unwrap_or_else(|| panic_with_error!(&env, FactoryError::Unauthorized));
        admin.require_auth();
        set_operator(&env, &new_operator);
        #[allow(deprecated)]
        env.events()
            .publish((ROLE_GRANTED, symbol_short!("operator")), (new_operator,));
    }

    pub fn deploy_username(
        env: Env,
        username_hash: BytesN<32>,
        owner: Address,
    ) -> Result<(), FactoryError> {
        
        let auction_contract = read_auction_contract(&env).ok_or(FactoryError::Unauthorized)?;
        auction_contract.require_auth();

        if has_username(&env, &username_hash) {
            return Err(FactoryError::AlreadyDeployed);
        }

        let core_contract =
            read_core_contract(&env , username_hash.clone()).ok_or(FactoryError::CoreContractNotConfigured)?;

        let record = UsernameRecord {
            username_hash: username_hash.clone(),
            owner,
            registered_at: env.ledger().timestamp(),
            core_contract,
        };

        set_username(&env, &record.username_hash.clone(), &record);
        emit_username_deployed(
            &env,
            &record.username_hash,
            &record.owner,
            record.registered_at,
        );
        Ok(())
    }

    pub fn transfer_username(
        env: Env,
        username_hash: BytesN<32>,
        new_owner: Address,
    ) -> Result<(), FactoryError> {
        let auction_contract = read_auction_contract(&env).ok_or(FactoryError::Unauthorized)?;
        auction_contract.require_auth();

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

    pub fn get_username_owner(env: Env, username_hash: BytesN<32>) -> Option<Address> {
        get_username(&env, &username_hash).map(|r| r.owner)
    }

    pub fn auction_contract(env: Env) -> Option<Address> {
        read_auction_contract(&env)
    }

    pub fn core_contract(env: Env , username_hash: BytesN<32>) -> Option<Address> {
        read_core_contract(&env , username_hash)
    }
}
