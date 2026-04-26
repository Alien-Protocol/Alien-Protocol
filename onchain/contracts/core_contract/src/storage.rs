use soroban_sdk::{contracttype, Address, BytesN, Env};

use crate::types::PrivacyMode;

/// The amount of ledger entries to bump persistent storage by.
pub(crate) const PERSISTENT_BUMP_AMOUNT: u32 = 518_400;
/// The threshold for persistent storage TTL to trigger an auto-bump.
pub(crate) const PERSISTENT_LIFETIME_THRESHOLD: u32 = 120_960;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Resolver(BytesN<32>),
    SmtRoot,
    StellarAddress(BytesN<32>),
    StellarAddresses(BytesN<32>),
    PrivacyMode(BytesN<32>),
    /// The contract owner.
    Owner,
    /// The contract admin.
    Admin,
    /// The contract operator.
    Operator,
    ShieldedAddress(BytesN<32>),
    CreatedAt(BytesN<32>),
    Delegate(BytesN<32>, Address),
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

/// Sets the contract owner.
pub fn set_owner(env: &Env, owner: &Address) {
    env.storage().instance().set(&DataKey::Owner, owner);
}

/// Returns the contract owner.
pub fn get_owner(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Owner)
}

/// Sets the contract admin.
pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

/// Returns the contract admin.
pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Admin)
}

/// Sets the contract operator.
pub fn set_operator(env: &Env, operator: &Address) {
    env.storage().instance().set(&DataKey::Operator, operator);
}

/// Returns the contract operator.
pub fn get_operator(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Operator)
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

pub fn set_delegate_permissions(
    env: &Env,
    username_hash: &BytesN<32>,
    delegate: &Address,
    permissions: &crate::types::PermissionSet,
) {
    let key = DataKey::Delegate(username_hash.clone(), delegate.clone());
    env.storage().persistent().set(&key, permissions);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn get_delegate_permissions(
    env: &Env,
    username_hash: &BytesN<32>,
    delegate: &Address,
) -> Option<crate::types::PermissionSet> {
    env.storage()
        .persistent()
        .get(&DataKey::Delegate(username_hash.clone(), delegate.clone()))
}

pub fn remove_delegate_permissions(env: &Env, username_hash: &BytesN<32>, delegate: &Address) {
    env.storage()
        .persistent()
        .remove(&DataKey::Delegate(username_hash.clone(), delegate.clone()));
}

pub fn has_permission(
    env: &Env,
    username_hash: &BytesN<32>,
    caller: &Address,
    permission: crate::types::Permission,
) -> bool {
    if let Some(permissions) = get_delegate_permissions(env, username_hash, caller) {
        permissions.permissions.contains(&permission)
    } else {
        false
    }
}
