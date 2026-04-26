use soroban_sdk::{contracttype, Address, BytesN, Env};

use crate::types::PrivacyMode;

/// Number of ledgers to bump persistent storage entries by.
pub(crate) const PERSISTENT_BUMP_AMOUNT: u32 = 518_400;
/// Minimum remaining ledgers before a persistent entry is bumped.
pub(crate) const PERSISTENT_LIFETIME_THRESHOLD: u32 = 120_960;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Resolver(BytesN<32>),
    SmtRoot,
    StellarAddress(BytesN<32>),
    StellarAddresses(BytesN<32>),
    PrivacyMode(BytesN<32>),
    Owner,
    ShieldedAddress(BytesN<32>),
    CreatedAt(BytesN<32>),
}

pub fn set_privacy_mode(env: &Env, username_hash: &BytesN<32>, mode: &PrivacyMode) {
    let key = DataKey::PrivacyMode(username_hash.clone());
    env.storage().persistent().set(&key, mode);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn get_privacy_mode(env: &Env, username_hash: &BytesN<32>) -> PrivacyMode {
    env.storage()
        .persistent()
        .get::<DataKey, PrivacyMode>(&DataKey::PrivacyMode(username_hash.clone()))
        .unwrap_or(PrivacyMode::Normal)
}

pub fn set_owner(env: &Env, owner: &Address) {
    env.storage().instance().set(&DataKey::Owner, owner);
}

pub fn get_owner(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Owner)
}

pub fn is_initialized(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Owner)
}

pub fn set_shielded_address(env: &Env, username_hash: &BytesN<32>, commitment: &BytesN<32>) {
    let key = DataKey::ShieldedAddress(username_hash.clone());
    env.storage().persistent().set(&key, commitment);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn get_shielded_address(env: &Env, username_hash: &BytesN<32>) -> Option<BytesN<32>> {
    env.storage()
        .persistent()
        .get(&DataKey::ShieldedAddress(username_hash.clone()))
}

pub fn has_shielded_address(env: &Env, username_hash: &BytesN<32>) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::ShieldedAddress(username_hash.clone()))
}

pub fn set_created_at(env: &Env, username_hash: &BytesN<32>, timestamp: u64) {
    let key = DataKey::CreatedAt(username_hash.clone());
    env.storage().persistent().set(&key, &timestamp);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn get_created_at(env: &Env, username_hash: &BytesN<32>) -> Option<u64> {
    env.storage()
        .persistent()
        .get(&DataKey::CreatedAt(username_hash.clone()))
}
