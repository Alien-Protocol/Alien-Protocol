use soroban_sdk::{Address, BytesN, Env};

use crate::types::{DataKey, DeployConfig, UsernameRecord};

/// Number of ledgers to bump persistent storage entries by.
pub(crate) const PERSISTENT_BUMP_AMOUNT: u32 = 518_400;
/// Minimum remaining ledgers before a persistent entry is bumped.
pub(crate) const PERSISTENT_LIFETIME_THRESHOLD: u32 = 120_960;

/// Stores the auction contract address in instance storage.
pub fn set_auction_contract(env: &Env, auction_contract: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::AuctionContract, auction_contract);
}

/// Retrieves the auction contract address from instance storage.
pub fn get_auction_contract(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::AuctionContract)
}

/// Stores the core contract address in instance storage.
pub fn set_core_contract(env: &Env, core_contract: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::CoreContract, core_contract);
}

/// Retrieves the core contract address from instance storage.
pub fn get_core_contract(env: &Env) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::CoreContract)
}

/// Stores a username record in persistent storage under its hash key.
pub fn set_username(env: &Env, hash: &BytesN<32>, record: &UsernameRecord) {
    let key = DataKey::Username(hash.clone());
    env.storage().persistent().set(&key, record);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

/// Retrieves a username record from persistent storage by its hash.
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

/// Returns true if a username record exists for the given hash.
pub fn has_username(env: &Env, hash: &BytesN<32>) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Username(hash.clone()))
}

/// Retrieves the deploy configuration from persistent storage.
#[allow(dead_code)]
pub fn get_config(env: &Env) -> Option<DeployConfig> {
    env.storage()
        .persistent()
        .get::<DataKey, DeployConfig>(&DataKey::Config)
}

/// Stores the deploy configuration in persistent storage.
#[allow(dead_code)]
pub fn set_config(env: &Env, config: &DeployConfig) {
    let key = DataKey::Config;
    env.storage().persistent().set(&key, config);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}
