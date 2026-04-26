use soroban_sdk::{Address, BytesN, Env};

use crate::types::{DataKey, DeployConfig, UsernameRecord};

/// The amount of ledger entries to bump persistent storage by.
pub(crate) const PERSISTENT_BUMP_AMOUNT: u32 = 518_400;
/// The threshold for persistent storage TTL to trigger an auto-bump.
pub(crate) const PERSISTENT_LIFETIME_THRESHOLD: u32 = 120_960;

/// Sets the auction contract address.
pub fn set_auction_contract(env: &Env, auction_contract: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::AuctionContract, auction_contract);
}

/// Returns the auction contract address.
pub fn get_auction_contract(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::AuctionContract)
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
    env.storage()
        .instance()
        .set(&DataKey::CoreContract, core_contract);
}

/// Returns the core contract address.
pub fn get_core_contract(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::CoreContract)
}

/// Stores a username record.
pub fn set_username(env: &Env, hash: &BytesN<32>, record: &UsernameRecord) {
    let key = DataKey::Username(hash.clone());
    env.storage().persistent().set(&key, record);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

/// Returns a username record.
pub fn get_username(env: &Env, hash: &BytesN<32>) -> Option<UsernameRecord> {
    let key = DataKey::Username(hash.clone());
    let record = env
        .storage()
        .persistent()
        .get::<DataKey, UsernameRecord>(&key);
    if record.is_some() {
        env.storage().persistent().extend_ttl(
            &key,
            PERSISTENT_LIFETIME_THRESHOLD,
            PERSISTENT_BUMP_AMOUNT,
        );
    }
    record
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
    env.storage()
        .persistent()
        .get::<DataKey, DeployConfig>(&DataKey::Config)
}

/// Stores the deploy configuration in persistent storage.
#[allow(dead_code)]
/// Sets the deployment configuration.
pub fn set_config(env: &Env, config: &DeployConfig) {
    let key = DataKey::Config;
    env.storage().persistent().set(&key, config);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}
