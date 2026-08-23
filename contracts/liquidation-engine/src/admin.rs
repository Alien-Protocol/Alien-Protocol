use soroban_sdk::{Address, Env};

use crate::errors::EngineError;
use crate::events::{AdminChanged, Initialized};
use crate::storage;

pub fn initialize(
    env: Env,
    admin: Address,
    vault: Address,
    pool: Address,
    oracle: Address,
) -> Result<(), EngineError> {
    if storage::has_admin(&env) {
        return Err(EngineError::AlreadyInitialized);
    }

    admin.require_auth();
    if admin == vault
        || admin == pool
        || admin == oracle
        || vault == pool
        || vault == oracle
        || pool == oracle
    {
        return Err(EngineError::InvalidAddress);
    }

    // Store all four addresses
    storage::set_admin(&env, &admin);
    storage::set_vault(&env, &vault);
    storage::set_pool(&env, &pool);
    storage::set_oracle(&env, &oracle);

    Initialized {
        admin,
        vault,
        pool,
        oracle,
    }
    .publish(&env);

    Ok(())
}

pub fn set_admin(env: Env, new_admin: Address) -> Result<(), EngineError> {
    let current_admin = storage::get_admin(&env).ok_or(EngineError::NotInitialized)?;

    if current_admin == new_admin {
        return Err(EngineError::AlreadyAdmin);
    }

    // Validate new_admin doesn't collide with vault, pool, or oracle
    let vault = storage::get_vault(&env);
    let pool = storage::get_pool(&env);
    let oracle = storage::get_oracle(&env);

    if (vault.is_some() && new_admin == vault.unwrap())
        || (pool.is_some() && new_admin == pool.unwrap())
        || (oracle.is_some() && new_admin == oracle.unwrap())
    {
        return Err(EngineError::InvalidAddress);
    }

    current_admin.require_auth();

    storage::set_admin(&env, &new_admin);

    AdminChanged {
        old_admin: current_admin,
        new_admin,
    }
    .publish(&env);

    Ok(())
}

pub fn set_vault(env: Env, vault: Address) -> Result<(), EngineError> {
    let admin = storage::get_admin(&env).ok_or(EngineError::NotInitialized)?;

    // Validate vault doesn't collide with admin, pool, or oracle
    let current_pool = storage::get_pool(&env);
    let current_oracle = storage::get_oracle(&env);

    if vault == admin
        || (current_pool.is_some() && vault == current_pool.unwrap())
        || (current_oracle.is_some() && vault == current_oracle.unwrap())
    {
        return Err(EngineError::InvalidAddress);
    }

    admin.require_auth();

    storage::set_vault(&env, &vault);

    Ok(())
}

pub fn set_pool(env: Env, pool: Address) -> Result<(), EngineError> {
    let admin = storage::get_admin(&env).ok_or(EngineError::NotInitialized)?;

    // Validate pool doesn't collide with admin, vault, or oracle
    let current_vault = storage::get_vault(&env);
    let current_oracle = storage::get_oracle(&env);

    if pool == admin
        || (current_vault.is_some() && pool == current_vault.unwrap())
        || (current_oracle.is_some() && pool == current_oracle.unwrap())
    {
        return Err(EngineError::InvalidAddress);
    }

    admin.require_auth();

    storage::set_pool(&env, &pool);

    Ok(())
}
