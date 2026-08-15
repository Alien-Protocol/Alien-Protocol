use soroban_sdk::{Address, Env};

use crate::errors::PoolError;

#[soroban_sdk::contractclient(name = "VaultClient")]
#[allow(dead_code)]
pub trait Vault {
    fn get_collateral_value(env: Env, user: Address) -> i128;
    fn get_health_factor(env: Env, user: Address) -> i128;
}

pub fn borrow(env: Env, user: Address, asset: Address, amount: i128) -> Result<(), PoolError> {
    let _ = (env, user, asset, amount);
    Err(PoolError::NotImplemented)
}

pub fn calculate_limit(env: Env, user: Address) -> Result<i128, PoolError> {
    let _ = (env, user);
    Err(PoolError::NotImplemented)
}
