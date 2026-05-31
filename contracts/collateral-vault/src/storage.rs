use crate::types::{CollateralAsset, DataKey, Position};
use soroban_sdk::{Address, Env, Vec};

pub fn get_position(env: &Env, user: &Address, asset: &Address) -> Result<Position, VaultError> {
    let key = Datakey::Position(user.clone(), asset.clone());
    env.storage()
        .persistent()
        .get(&key)
        .ok_or(VaultError::NoPosition)
}

pub fn set_position(env: &Env, user: &Address, asset: &Address, position: &Position) {
    let key = Datakey::Position(user.clone(), asset.clone());
    env.storage().persistent().set(&key, position);
}

pub fn remove_position(env: &Env, user: &Address, asset: &Address) {
    let key = Datakey::Position(user.clone(), asset.clone());
    env.storage().persistent().remove(&key);
}

pub fn get_position_index(env: &Env) -> Map<(Address, Address), i128> {
    let key = Datakey::PositionIndex;
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or(Map::new(env))
}

pub fn set_position_index(env: &Env, index: &Map<(Address, Address), i128>) {
    let key = Datakey::PositionIndex;
    env.storage().persistent().set(&key, index);
}

pub fn update_position_index(env: &Env, user: &Address, asset: &Address, amount: i128) {
    let mut index = get_position_index(env);
    let key = (user.clone(), asset.clone());
    if amount == 0 {
        index.remove(key);
    } else {
        index.set(key, amount);
    }
    set_position_index(env, &index);
}

pub fn get_oracle(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::Oracle)
}

pub fn set_oracle(env: &Env, oracle: &Address) {
    env.storage().persistent().set(&DataKey::Oracle, oracle);
}

pub fn get_liquidation_engine(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::LiquidationEngine)
}

pub fn set_liquidation_engine(env: &Env, engine: &Address) {
    env.storage()
        .persistent()
        .set(&DataKey::LiquidationEngine, engine);
}

pub fn get_pool(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::Pool)
}

pub fn set_pool(env: &Env, pool: &Address) {
    env.storage().persistent().set(&DataKey::Pool, pool);
}

pub fn get_position_balance(env: &Env, user: &Address, asset: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::Position(user.clone(), asset.clone()))
        .unwrap_or(0)
}

pub fn set_lending_pool(env: &Env, address: &Address) {
    let key = Datakey::LendingPool;
    env.storage().persistent().set(&key, address);
}

pub fn is_paused(env: &Env) -> bool {
    let key = Datakey::Paused;
    env.storage().persistent().get(&key).unwrap_or(false)
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

pub fn set_admin(env: &Env, admin: &Address) {
    let key = Datakey::Admin;
    env.storage().persistent().set(&key, admin);
}

/// Build a Position for a user by loading all their non-zero balances.
pub fn get_position(env: &Env, user: &Address) -> Option<Position> {
    let index = get_position_index(env);
    if !index.contains(user) {
        return None;
    }

    let all_assets = get_user_assets(env, user);
    let mut collateral: Vec<CollateralAsset> = Vec::new(env);

    for asset in all_assets.iter() {
        let balance = get_position_balance(env, user, &asset);
        if balance > 0 {
            collateral.push_back(CollateralAsset {
                asset: asset.clone(),
                amount: balance,
            });
        }
    }

    if collateral.is_empty() {
        return None;
    }

    Some(Position {
        user: user.clone(),
        collateral,
    })
}

/// Returns all active positions (users with at least one non-zero balance).
pub fn get_all_positions(env: &Env) -> Vec<Position> {
    let index = get_position_index(env);
    let mut positions: Vec<Position> = Vec::new(env);
    for user in index.iter() {
        if let Some(position) = get_position(env, &user) {
            positions.push_back(position);
        }
    }
    positions
}
