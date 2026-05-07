use shared::storage as shared_storage;
use soroban_sdk::{Address, BytesN, Env};

use crate::types::{DataKey, DeployConfig, UsernameRecord};

<<<<<<< HEAD
/// TTL constants for persistent storage entries.
/// Bump amount: ~30 days (at ~5s per ledger close).
#[allow(dead_code)]
pub(crate) const PERSISTENT_BUMP_AMOUNT: u32 = 518_400;
/// Lifetime threshold: ~7 days — entries are extended when remaining TTL drops below this.
#[allow(dead_code)]
pub(crate) const PERSISTENT_LIFETIME_THRESHOLD: u32 = 120_960;

/// Sets the auction contract address.
pub fn set_auction_contract(env: &Env, auction_contract: &Address) {
    shared_storage::set_instance(env, &DataKey::AuctionContract, auction_contract);
}

/// Returns the auction contract address.
pub fn get_auction_contract(env: &Env) -> Option<Address> {
    shared_storage::get_instance(env, &DataKey::AuctionContract)
}

/// Sets the contract owner.
=======
pub(crate) const PERSISTENT_BUMP_AMOUNT: u32 = 518_400;
pub(crate) const PERSISTENT_LIFETIME_THRESHOLD: u32 = 120_960;
>>>>>>> 5c8a9fb (refactor: full codebase)
pub fn set_owner(env: &Env, owner: &Address) {
    env.storage().instance().set(&DataKey::Owner, owner);
}

pub fn get_owner(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::Owner)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::Admin)
}

pub fn set_operator(env: &Env, operator: &Address) {
    env.storage().instance().set(&DataKey::Operator, operator);
}

pub fn get_operator(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::Operator)
}

<<<<<<< HEAD
/// Sets the core contract address.
pub fn set_core_contract(env: &Env, core_contract: &Address) {
    shared_storage::set_instance(env, &DataKey::CoreContract, core_contract);
}

/// Returns the core contract address.
pub fn get_core_contract(env: &Env) -> Option<Address> {
    shared_storage::get_instance(env, &DataKey::CoreContract)
=======
pub fn set_core_contract(env: &Env, username: BytesN<32>, core_contract: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::CoreContract(username), core_contract);
}

pub fn get_core_contract(env: &Env, username: BytesN<32>) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::CoreContract(username))
>>>>>>> 5c8a9fb (refactor: full codebase)
}

pub fn set_username(env: &Env, hash: &BytesN<32>, record: &UsernameRecord) {
    let key = DataKey::Username(hash.clone());
    shared_storage::set_persistent(env, &key, record);
}

pub fn get_username(env: &Env, hash: &BytesN<32>) -> Option<UsernameRecord> {
    let key = DataKey::Username(hash.clone());
    shared_storage::get_persistent_with_ttl(env, &key)
}

#[allow(dead_code)]
pub fn get_config(env: &Env) -> Option<DeployConfig> {
    shared_storage::get_persistent(env, &DataKey::Config)
}

#[allow(dead_code)]
pub fn set_config(env: &Env, config: &DeployConfig) {
    let key = DataKey::Config;
    shared_storage::set_persistent(env, &key, config);
}

pub fn set_core_wasm_hash(env: &Env, wasm_hash: &BytesN<32>) {
    let key = DataKey::CoreWasm;
    env.storage().persistent().set(&key, wasm_hash);
}

pub fn get_core_wasm_hash(env: &Env) -> Option<BytesN<32>> {
    let key = DataKey::CoreWasm;
    env.storage().persistent().get(&key)
}
