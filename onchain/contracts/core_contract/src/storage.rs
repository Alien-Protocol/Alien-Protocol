use soroban_sdk::{Address, BytesN, Env, Symbol, Vec};

use crate::types::{DataKey, EscrowRecord, WalletEntry};

pub(crate) const PERSISTENT_BUMP: u32 = 518_400;
pub(crate) const PERSISTENT_THRESHOLD: u32 = 120_960;

fn bump_persistent<K>(env: &Env, key: &K)
where
    K: soroban_sdk::TryFromVal<Env, soroban_sdk::Val> + soroban_sdk::IntoVal<Env, soroban_sdk::Val>,
{
    env.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_THRESHOLD, PERSISTENT_BUMP);
}

pub fn set_owner(env: &Env, owner: &Address) {
    env.storage().instance().set(&DataKey::Owner, owner);
}

pub fn get_owner(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Owner)
}

pub fn set_username_hash(env: &Env, hash: &BytesN<32>) {
    env.storage().instance().set(&DataKey::UsernameHash, hash);
}

pub fn get_username_hash(env: &Env) -> Option<BytesN<32>> {
    env.storage().instance().get(&DataKey::UsernameHash)
}

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
