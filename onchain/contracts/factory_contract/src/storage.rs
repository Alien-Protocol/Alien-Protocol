use soroban_sdk::{Address, BytesN, Env};

use crate::types::{DataKey, DeployConfig, UsernameRecord};

pub(crate) const PERSISTENT_BUMP_AMOUNT: u32 = 518_400;
pub(crate) const PERSISTENT_LIFETIME_THRESHOLD: u32 = 120_960;
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

pub fn set_core_contract(env: &Env, username: BytesN<32>, core_contract: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::CoreContract(username), core_contract);
}

pub fn get_core_contract(env: &Env, username: BytesN<32>) -> Option<Address> {
    env.storage()
        .instance()
        .get::<DataKey, Address>(&DataKey::CoreContract(username))
}

pub fn set_username(env: &Env, hash: &BytesN<32>, record: &UsernameRecord) {
    let key = DataKey::Username(hash.clone());
    env.storage().persistent().set(&key, record);
    env.storage().persistent().extend_ttl(
        &key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

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

#[allow(dead_code)]
pub fn get_config(env: &Env) -> Option<DeployConfig> {
    env.storage()
        .persistent()
        .get::<DataKey, DeployConfig>(&DataKey::Config)
}

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

pub fn set_core_wasm_hash(env: &Env, wasm_hash: &BytesN<32>) {
    let key = DataKey::CoreWasm;
    env.storage().persistent().set(&key, wasm_hash);
}

pub fn get_core_wasm_hash(env: &Env) -> Option<BytesN<32>> {
    let key = DataKey::CoreWasm;
    env.storage().persistent().get(&key)
}
