use soroban_sdk::{Address, Env};

use crate::errors::PoolError;

pub fn repay(env: Env, user: Address, asset: Address, amount: i128) -> Result<(), PoolError> {
    let _ = (env, user, asset, amount);
    Err(PoolError::NotImplemented)
}

pub fn repay_for(
    env: Env,
    payer: Address,
    user: Address,
    asset: Address,
    amount: i128,
) -> Result<(), PoolError> {
    let _ = (env, payer, user, asset, amount);
    Err(PoolError::NotImplemented)
}

pub fn is_liquidatable(env: Env, user: Address) -> Result<bool, PoolError> {
    let _ = (env, user);
    Err(PoolError::NotImplemented)
}
