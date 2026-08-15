#![allow(dead_code)]
use soroban_sdk::{Address, Env};

use crate::types::{DataKey, Debt, PauseFlag};

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

pub fn get_oracle(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::Oracle)
}

pub fn set_oracle(env: &Env, oracle: &Address) {
    env.storage().persistent().set(&DataKey::Oracle, oracle);
}

pub fn get_borrow_asset(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::BorrowAsset)
}

pub fn set_borrow_asset(env: &Env, asset: &Address) {
    env.storage().persistent().set(&DataKey::BorrowAsset, asset);
}

pub fn get_interest_rate_bps(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::InterestRateBps)
        .unwrap_or(0)
}

pub fn set_interest_rate_bps(env: &Env, rate: u32) {
    env.storage()
        .persistent()
        .set(&DataKey::InterestRateBps, &rate);
}

pub fn get_pause_mask(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::PauseMask)
        .unwrap_or(0)
}

pub fn set_pause_mask(env: &Env, mask: u32) {
    env.storage().persistent().set(&DataKey::PauseMask, &mask);
}

pub fn is_operation_paused(env: &Env, flag: &PauseFlag) -> bool {
    get_pause_mask(env) & flag.bit() != 0
}

pub fn get_total_supply(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::TotalSupply)
        .unwrap_or(0)
}

pub fn set_total_supply(env: &Env, amount: i128) {
    env.storage()
        .persistent()
        .set(&DataKey::TotalSupply, &amount);
}

pub fn get_total_borrowed(env: &Env) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::TotalBorrowed)
        .unwrap_or(0)
}

pub fn set_total_borrowed(env: &Env, amount: i128) {
    env.storage()
        .persistent()
        .set(&DataKey::TotalBorrowed, &amount);
}

pub fn get_user_supply(env: &Env, user: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::Supply(user.clone()))
        .unwrap_or(0)
}

pub fn set_user_supply(env: &Env, user: &Address, amount: i128) {
    env.storage()
        .persistent()
        .set(&DataKey::Supply(user.clone()), &amount);
}

pub fn get_debt(env: &Env, user: &Address) -> Option<Debt> {
    env.storage().persistent().get(&DataKey::Debt(user.clone()))
}

pub fn set_debt(env: &Env, user: &Address, debt: &Debt) {
    env.storage()
        .persistent()
        .set(&DataKey::Debt(user.clone()), debt);
}

pub fn remove_debt(env: &Env, user: &Address) {
    env.storage()
        .persistent()
        .remove(&DataKey::Debt(user.clone()));
}
