use crate::errors::VaultError;
use crate::types::{Datakey, Position};
use soroban_sdk::{Address, Env, Map};

pub fn get_position(env: &Env, user: &Address) -> Result<Position, VaultError> {
    let key = Datakey::Position(user.clone());
    env.storage()
        .persistent()
        .get(&key)
        .ok_or(VaultError::NoPosition)
}

pub fn set_position(env: &Env, user: &Address, position: &Position) {
    let key = Datakey::Position(user.clone());
    env.storage().persistent().set(&key, position);
}

pub fn remove_position(env: &Env, user: &Address) {
    let key = Datakey::Position(user.clone());
    env.storage().persistent().remove(&key);
}

pub fn get_position_index(env: &Env) -> Map<Address, i128> {
    let key = Datakey::PositionIndex;
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or(Map::new(env))
}

pub fn set_position_index(env: &Env, index: &Map<Address, i128>) {
    let key = Datakey::PositionIndex;
    env.storage().persistent().set(&key, index);
}

pub fn update_position_index(env: &Env, user: &Address, amount: i128) {
    let mut index = get_position_index(env);
    if amount == 0 {
        index.remove(user.clone());
    } else {
        index.set(user.clone(), amount);
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

pub fn is_paused(env: &Env) -> bool {
    let key = Datakey::Paused;
    env.storage().persistent().get(&key).unwrap_or(false)
}
