<<<<<<< HEAD
use shared::storage as shared_storage;
use soroban_sdk::{contracttype, Address, BytesN, Env};
=======
use soroban_sdk::{Address, BytesN, Env, Symbol, Vec};
>>>>>>> 5c8a9fb (refactor: full codebase)

use crate::types::{DataKey, EscrowRecord, WalletEntry};

pub(crate) const PERSISTENT_BUMP: u32 = 518_400;
pub(crate) const PERSISTENT_THRESHOLD: u32 = 120_960;

<<<<<<< HEAD
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
    shared_storage::set_persistent(env, &key, mode);
}

pub fn get_privacy_mode(env: &Env, username_hash: &BytesN<32>) -> PrivacyMode {
=======
fn bump_persistent<K>(env: &Env, key: &K)
where
    K: soroban_sdk::TryFromVal<Env, soroban_sdk::Val>
        + soroban_sdk::IntoVal<Env, soroban_sdk::Val>,
{
>>>>>>> 5c8a9fb (refactor: full codebase)
    env.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_THRESHOLD, PERSISTENT_BUMP);
}

pub fn set_owner(env: &Env, owner: &Address) {
    shared_storage::set_instance(env, &DataKey::Owner, owner);
}

pub fn get_owner(env: &Env) -> Option<Address> {
    shared_storage::get_instance(env, &DataKey::Owner)
}

pub fn set_username_hash(env: &Env, hash: &BytesN<32>) {
    env.storage().instance().set(&DataKey::UsernameHash, hash);
}

pub fn get_username_hash(env: &Env) -> Option<BytesN<32>> {
    env.storage().instance().get(&DataKey::UsernameHash)
}

<<<<<<< HEAD
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
    shared_storage::set_persistent(env, &key, commitment);
}

pub fn get_shielded_address(env: &Env, username_hash: &BytesN<32>) -> Option<BytesN<32>> {
    shared_storage::get_persistent(env, &DataKey::ShieldedAddress(username_hash.clone()))
}

pub fn has_shielded_address(env: &Env, username_hash: &BytesN<32>) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::ShieldedAddress(username_hash.clone()))
}

pub fn set_created_at(env: &Env, username_hash: &BytesN<32>, timestamp: u64) {
    let key = DataKey::CreatedAt(username_hash.clone());
    shared_storage::set_persistent(env, &key, &timestamp);
}

pub fn get_created_at(env: &Env, username_hash: &BytesN<32>) -> Option<u64> {
    shared_storage::get_persistent(env, &DataKey::CreatedAt(username_hash.clone()))
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
=======
pub fn set_primary_address(env: &Env, addr: &Address) {
    env.storage()
        .persistent()
        .set(&DataKey::PrimaryAddress, addr);
    bump_persistent(env, &DataKey::PrimaryAddress);
}

pub fn get_primary_address(env: &Env) -> Option<Address> {
    let opt: Option<Address> = env.storage().persistent().get(&DataKey::PrimaryAddress);
    if opt.is_some() {
        bump_persistent(env, &DataKey::PrimaryAddress);
>>>>>>> 5c8a9fb (refactor: full codebase)
    }
    opt
}

pub fn set_wallet(env: &Env, label: &Symbol, entry: &WalletEntry) {
    let key = DataKey::Wallet(label.clone());
    env.storage().persistent().set(&key, entry);
    bump_persistent(env, &key);
}

pub fn get_wallet(env: &Env, label: &Symbol) -> Option<WalletEntry> {
    let key = DataKey::Wallet(label.clone());
    let opt: Option<WalletEntry> = env.storage().persistent().get(&key);
    if opt.is_some() {
        bump_persistent(env, &key);
    }
    opt
}

pub fn remove_wallet(env: &Env, label: &Symbol) {
    env.storage()
        .persistent()
        .remove(&DataKey::Wallet(label.clone()));
}

pub fn has_wallet(env: &Env, label: &Symbol) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Wallet(label.clone()))
}

pub fn get_wallet_labels(env: &Env) -> Vec<Symbol> {
    let opt: Option<Vec<Symbol>> = env.storage().persistent().get(&DataKey::WalletLabels);
    if let Some(ref labels) = opt {
        if !labels.is_empty() {
            bump_persistent(env, &DataKey::WalletLabels);
        }
    }
    opt.unwrap_or_else(|| Vec::new(env))
}

pub fn set_wallet_labels(env: &Env, labels: &Vec<Symbol>) {
    env.storage()
        .persistent()
        .set(&DataKey::WalletLabels, labels);
    bump_persistent(env, &DataKey::WalletLabels);
}
pub fn get_escrow_counter(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::EscrowCounter)
        .unwrap_or(0)
}

pub fn set_escrow_counter(env: &Env, counter: u32) {
    env.storage()
        .instance()
        .set(&DataKey::EscrowCounter, &counter);
}

pub fn set_escrow(env: &Env, id: u32, record: &EscrowRecord) {
    let key = DataKey::Escrow(id);
    env.storage().persistent().set(&key, record);
    bump_persistent(env, &key);
}

pub fn get_escrow(env: &Env, id: u32) -> Option<EscrowRecord> {
    let key = DataKey::Escrow(id);
    let opt: Option<EscrowRecord> = env.storage().persistent().get(&key);
    if opt.is_some() {
        bump_persistent(env, &key);
    }
    opt
}
