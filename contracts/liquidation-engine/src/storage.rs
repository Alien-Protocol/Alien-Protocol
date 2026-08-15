#![allow(dead_code)]
use soroban_sdk::{Address, Env};

use crate::types::DataKey;

pub fn has_admin(env: &Env) -> bool {
    env.storage().persistent().has(&DataKey::Admin)
}

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::Admin)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().persistent().set(&DataKey::Admin, admin);
}

pub fn get_vault(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::Vault)
}

pub fn set_vault(env: &Env, vault: &Address) {
    env.storage().persistent().set(&DataKey::Vault, vault);
}

pub fn get_pool(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::Pool)
}

pub fn set_pool(env: &Env, pool: &Address) {
    env.storage().persistent().set(&DataKey::Pool, pool);
}

pub fn get_oracle(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::Oracle)
}

pub fn set_oracle(env: &Env, oracle: &Address) {
    env.storage().persistent().set(&DataKey::Oracle, oracle);
}
