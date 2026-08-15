use soroban_sdk::{Address, Env};

use crate::errors::PoolError;

pub fn supply(env: Env, user: Address, amount: i128) -> Result<(), PoolError> {
    let _ = (env, user, amount);
    Err(PoolError::NotImplemented)
}

pub fn withdraw_liquidity(env: Env, user: Address, amount: i128) -> Result<(), PoolError> {
    let _ = (env, user, amount);
    Err(PoolError::NotImplemented)
}

pub fn get_user_supply(env: Env, user: Address) -> i128 {
    crate::storage::get_user_supply(&env, &user)
}

pub fn get_total_supply(env: Env) -> i128 {
    crate::storage::get_total_supply(&env)
}

pub fn get_available_liquidity(env: Env) -> i128 {
    let _ = env;
    0
}

pub fn get_utilization_bps(env: Env) -> u32 {
    let _ = env;
    0
}
