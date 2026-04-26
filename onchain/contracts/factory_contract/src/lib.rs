#![no_std]

/// Error types for the factory contract.
mod errors;
/// Event emission helpers for the factory contract.
mod events;
/// Storage read/write helpers for the factory contract.
mod storage;
/// Shared types used across the factory contract.
mod types;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, panic_with_error, Address, BytesN, Env};

use crate::errors::FactoryError;
use crate::events::{emit_ownership_transferred, emit_username_deployed};
use crate::storage::{
    get_auction_contract as read_auction_contract, get_core_contract as read_core_contract,
    get_username, has_username, set_auction_contract, set_core_contract, set_username,
};
use crate::types::UsernameRecord;

#[contract]
pub struct FactoryContract;

#[contractimpl]
impl FactoryContract {
    pub fn configure(env: Env, auction_contract: Address, core_contract: Address) {
        set_auction_contract(&env, &auction_contract);
        set_core_contract(&env, &core_contract);
    }

    pub fn deploy_username(env: Env, username_hash: BytesN<32>, owner: Address) {
        let auction_contract = match read_auction_contract(&env) {
            Some(address) => address,
            None => panic_with_error!(&env, FactoryError::Unauthorized),
        };
        auction_contract.require_auth();

        if has_username(&env, &username_hash) {
            panic_with_error!(&env, FactoryError::AlreadyDeployed);
        }

        let core_contract = match read_core_contract(&env) {
            Some(address) => address,
            None => panic_with_error!(&env, FactoryError::CoreContractNotConfigured),
        };

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
    }

    pub fn transfer_username(env: Env, username_hash: BytesN<32>, new_owner: Address) {
        let auction_contract = match read_auction_contract(&env) {
            Some(address) => address,
            None => panic_with_error!(&env, FactoryError::Unauthorized),
        };
        auction_contract.require_auth();

        let mut record = get_username(&env, &username_hash).expect("Username not deployed");

        let old_owner = record.owner.clone();
        record.owner = new_owner.clone();

        set_username(&env, &username_hash, &record);
        emit_ownership_transferred(&env, &username_hash, &old_owner, &new_owner);
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

    pub fn core_contract(env: Env) -> Option<Address> {
        read_core_contract(&env)
    }
}
