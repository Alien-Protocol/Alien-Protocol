use soroban_sdk::{Address, Env};
use crate::types::{DataKey, Position};

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn get_admin(env: &Env) -> Address {
    env.storage().instance().get(&DataKey::Admin).expect("No admin set")
}

pub fn set_lending_pool(env: &Env, pool: &Address) {
    env.storage().instance().set(&DataKey::LendingPool, pool);
}

pub fn get_lending_pool(env: &Env) -> Address {
    env.storage().instance().get(&DataKey::LendingPool).expect("No lending pool set")
}

pub fn set_paused(env: &Env, paused: bool) {
    env.storage().instance().set(&DataKey::Paused, &paused);
}

pub fn get_paused(env: &Env) -> bool {
    env.storage().instance().get(&DataKey::Paused).unwrap_or(false)
}

pub fn set_supported_asset(env: &Env, asset: Address, is_supported: bool) {
    env.storage().instance().set(&DataKey::SupportedAsset(asset), &is_supported);
}

pub fn get_supported_asset(env: &Env, asset: Address) -> bool {
    env.storage().instance().get(&DataKey::SupportedAsset(asset)).unwrap_or(false)
}

pub fn set_position(env: &Env, user: Address, position: Position) {
    env.storage().instance().set(&DataKey::Position(user), &position);
}

pub fn get_position(env: &Env, user: Address) -> Option<Position> {
    env.storage().instance().get(&DataKey::Position(user))
}

pub fn set_position_index(env: &Env, index: u32) {
    env.storage().instance().set(&DataKey::PositionIndex, &index);
}

pub fn get_position_index(env: &Env) -> u32 {
    env.storage().instance().get(&DataKey::PositionIndex).unwrap_or(0)
}