use crate::errors::VaultError;
use crate::types::{Datakey, Position};
use soroban_sdk::{Address, Env, Map, Vec};

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

pub fn get_lending_pool(env: &Env) -> Option<Address> {
    let key = Datakey::LendingPool;
    env.storage().persistent().get(&key)
}

pub fn set_lending_pool(env: &Env, address: &Address) {
    let key = Datakey::LendingPool;
    env.storage().persistent().set(&key, address);
}

pub fn get_user_position_asset(env: &Env, user: &Address) -> Option<Address> {
    let index = get_position_index(env);
    let keys: Vec<(Address, Address)> = index.keys();
    let count = keys.len();
    let mut i = 0;
    while i < count {
        let key = keys.get(i).expect("position key missing");
        if key.0 == *user {
            return Some(key.1.clone());
        }
        i += 1;
    }
    None
}

pub fn get_user_position(env: &Env, user: &Address) -> Result<Position, VaultError> {
    let asset = get_user_position_asset(env, user).ok_or(VaultError::NoPosition)?;
    get_position(env, user, &asset)
}

pub fn get_liquidation_engine(env: &Env) -> Option<Address> {
    let key = Datakey::LiquidationEngine;
    env.storage().persistent().get(&key)
}

pub fn set_liquidation_engine(env: &Env, address: &Address) {
    let key = Datakey::LiquidationEngine;
    env.storage().persistent().set(&key, address);
}

pub fn is_paused(env: &Env) -> bool {
    let key = Datakey::Paused;
    env.storage().persistent().get(&key).unwrap_or(false)
}

pub fn get_admin(env: &Env) -> Option<Address> {
    let key = Datakey::Admin;
    env.storage().persistent().get(&key)
}

pub fn set_admin(env: &Env, admin: &Address) {
    let key = Datakey::Admin;
    env.storage().persistent().set(&key, admin);
}
