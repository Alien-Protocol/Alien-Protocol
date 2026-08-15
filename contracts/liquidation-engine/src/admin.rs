use soroban_sdk::{Address, Env};

use crate::errors::EngineError;

pub fn initialize(
    env: Env,
    admin: Address,
    vault: Address,
    pool: Address,
    oracle: Address,
) -> Result<(), EngineError> {
    let _ = (env, admin, vault, pool, oracle);
    Err(EngineError::NotImplemented)
}

pub fn set_admin(env: Env, new_admin: Address) -> Result<(), EngineError> {
    let _ = (env, new_admin);
    Err(EngineError::NotImplemented)
}

pub fn set_vault(env: Env, vault: Address) -> Result<(), EngineError> {
    let _ = (env, vault);
    Err(EngineError::NotImplemented)
}

pub fn set_pool(env: Env, pool: Address) -> Result<(), EngineError> {
    let _ = (env, pool);
    Err(EngineError::NotImplemented)
}
