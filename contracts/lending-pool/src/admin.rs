use soroban_sdk::{Address, Env, Symbol};

use crate::errors::PoolError;
use crate::types::PauseFlag;

pub fn initialize(
    env: Env,
    admin: Address,
    vault: Address,
    oracle: Address,
    borrow_asset: Address,
    interest_rate_bps: u32,
) -> Result<(), PoolError> {
    let _ = (env, admin, vault, oracle, borrow_asset, interest_rate_bps);
    Err(PoolError::NotImplemented)
}

pub fn set_admin(env: Env, new_admin: Address) -> Result<(), PoolError> {
    let _ = (env, new_admin);
    Err(PoolError::NotImplemented)
}

pub fn set_vault(env: Env, vault: Address) -> Result<(), PoolError> {
    let _ = (env, vault);
    Err(PoolError::NotImplemented)
}

pub fn set_oracle(env: Env, oracle: Address) -> Result<(), PoolError> {
    let _ = (env, oracle);
    Err(PoolError::NotImplemented)
}

pub fn pause_operation(env: Env, operation: PauseFlag, reason: Symbol) -> Result<(), PoolError> {
    let _ = (env, operation, reason);
    Err(PoolError::NotImplemented)
}

pub fn unpause_operation(env: Env, operation: PauseFlag) -> Result<(), PoolError> {
    let _ = (env, operation);
    Err(PoolError::NotImplemented)
}
