use crate::types::DataKey;
use soroban_sdk::{Address, Env, Vec};

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::Admin)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().persistent().set(&DataKey::Admin, admin);
}

pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::Paused)
        .unwrap_or(false)
}

pub fn set_paused(env: &Env, paused: bool) {
    env.storage().persistent().set(&DataKey::Paused, &paused);
}

pub fn pause(env: &Env) {
    set_paused(env, true);
}

pub fn is_supported_asset(env: &Env, asset: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::SupportedAsset(asset.clone()))
        .unwrap_or(false)
}

pub fn add_supported_asset(env: &Env, asset: &Address) {
    env.storage()
        .persistent()
        .set(&DataKey::SupportedAsset(asset.clone()), &true);
}

pub fn get_position_balance(env: &Env, user: &Address, asset: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::Position(user.clone(), asset.clone()))
        .unwrap_or(0)
}

pub fn set_position_balance(env: &Env, user: &Address, asset: &Address, balance: i128) {
    env.storage()
        .persistent()
        .set(&DataKey::Position(user.clone(), asset.clone()), &balance);
}

pub fn get_position_index(env: &Env) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::PositionIndex)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn add_to_position_index(env: &Env, user: &Address) {
    let mut index = get_position_index(env);
    if !index.contains(user) {
        index.push_back(user.clone());
        env.storage()
            .persistent()
            .set(&DataKey::PositionIndex, &index);
    }
}
