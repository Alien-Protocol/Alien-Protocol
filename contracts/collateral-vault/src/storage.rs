use crate::types::{DataKey, Position};
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

pub fn is_supported_asset(env: &Env, asset: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::SupportedAsset(asset.clone()))
        .unwrap_or(false)
}

pub fn get_supported_assets(env: &Env) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::SupportedAssets)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn add_supported_asset(env: &Env, asset: &Address) {
    env.storage()
        .persistent()
        .set(&DataKey::SupportedAsset(asset.clone()), &true);

    let mut assets = get_supported_assets(env);
    if !assets.contains(asset) {
        assets.push_back(asset.clone());
        env.storage()
            .persistent()
            .set(&DataKey::SupportedAssets, &assets);
    }
}

pub fn remove_supported_asset(env: &Env, asset: &Address) {
    env.storage()
        .persistent()
        .remove(&DataKey::SupportedAsset(asset.clone()));

    let mut assets = get_supported_assets(env);
    let mut found_idx = None;
    for i in 0..assets.len() {
        if assets.get(i).unwrap() == *asset {
            found_idx = Some(i);
            break;
        }
    }
    if let Some(idx) = found_idx {
        assets.remove(idx);
        env.storage()
            .persistent()
            .set(&DataKey::SupportedAssets, &assets);
    }
}

pub fn get_oracle(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::Oracle)
}

pub fn set_oracle(env: &Env, oracle: &Address) {
    env.storage().persistent().set(&DataKey::Oracle, oracle);
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


/// Remove a user from the position index (called when their balance reaches zero).
pub fn remove_from_position_index(env: &Env, user: &Address) {
    let index = get_position_index(env);
    let mut new_index: Vec<Address> = Vec::new(env);
    for addr in index.iter() {
        if &addr != user {
            new_index.push_back(addr);
        }
    }
    env.storage()
        .persistent()
        .set(&DataKey::PositionIndex, &new_index);
}

/// Track which assets a user has deposited into.

pub fn get_user_assets(env: &Env, user: &Address) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::UserAssets(user.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn add_user_asset(env: &Env, user: &Address, asset: &Address) {
    let mut assets = get_user_assets(env, user);
    if !assets.contains(asset) {
        assets.push_back(asset.clone());
        env.storage()
            .persistent()
            .set(&DataKey::UserAssets(user.clone()), &assets);
    }
}


/// Build a Position for a user by loading all their non-zero balances.
pub fn get_position(env: &Env, user: &Address) -> Position {
    let all_assets = get_user_assets(env, user);
    let mut active_assets: Vec<Address> = Vec::new(env);
    let mut active_balances: Vec<i128> = Vec::new(env);

    for asset in all_assets.iter() {
        let balance = get_position_balance(env, user, &asset);
        if balance > 0 {
            active_assets.push_back(asset.clone());
            active_balances.push_back(balance);
        }
    }

    Position {
        user: user.clone(),
        assets: active_assets,
        balances: active_balances,
    }
}

/// Returns all active positions (users with at least one non-zero balance).
pub fn get_all_positions(env: &Env) -> Vec<Position> {
    let index = get_position_index(env);
    let mut positions: Vec<Position> = Vec::new(env);
    for user in index.iter() {
        let position = get_position(env, &user);
        // Only include users that still have at least one active balance
        if !position.assets.is_empty() {
            positions.push_back(position);
        }
    }
    positions
}

