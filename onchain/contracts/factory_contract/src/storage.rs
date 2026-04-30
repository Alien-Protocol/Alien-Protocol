use shared::storage as shared_storage;
use soroban_sdk::{Address, BytesN, Env};

use crate::types::{DataKey, DeployConfig, UsernameRecord};

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
pub fn set_owner(env: &Env, owner: &Address) {
    env.storage().instance().set(&DataKey::Owner, owner);
}

/// Returns the contract owner.
pub fn get_owner(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::Owner)
}

/// Sets the contract admin.
pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

/// Returns the contract admin.
pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::Admin)
}

/// Sets the contract operator.
pub fn set_operator(env: &Env, operator: &Address) {
    env.storage().instance().set(&DataKey::Operator, operator);
}

/// Returns the contract operator.
pub fn get_operator(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::Operator)
}

/// Sets the core contract address.
pub fn set_core_contract(env: &Env, core_contract: &Address) {
    shared_storage::set_instance(env, &DataKey::CoreContract, core_contract);
}

/// Returns the core contract address.
pub fn get_core_contract(env: &Env) -> Option<Address> {
    shared_storage::get_instance(env, &DataKey::CoreContract)
}

/// Stores a username record.
pub fn set_username(env: &Env, hash: &BytesN<32>, record: &UsernameRecord) {
    let key = DataKey::Username(hash.clone());
    shared_storage::set_persistent(env, &key, record);
}

/// Returns a username record.
pub fn get_username(env: &Env, hash: &BytesN<32>) -> Option<UsernameRecord> {
    let key = DataKey::Username(hash.clone());
    shared_storage::get_persistent_with_ttl(env, &key)
}

/// Checks if a username hash is registered.
pub fn has_username(env: &Env, hash: &BytesN<32>) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Username(hash.clone()))
}

/// Retrieves the deploy configuration from persistent storage.
#[allow(dead_code)]
/// Returns the deployment configuration.
pub fn get_config(env: &Env) -> Option<DeployConfig> {
    shared_storage::get_persistent(env, &DataKey::Config)
}

/// Stores the deploy configuration in persistent storage.
#[allow(dead_code)]
/// Sets the deployment configuration.
pub fn set_config(env: &Env, config: &DeployConfig) {
    let key = DataKey::Config;
    shared_storage::set_persistent(env, &key, config);
}
