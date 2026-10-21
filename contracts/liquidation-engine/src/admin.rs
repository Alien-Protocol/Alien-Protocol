use soroban_sdk::{Address, Env};

use crate::errors::EngineError;
use crate::events::{AdminChanged, Initialized};
use crate::storage;

fn assert_distinct(candidate: &Address, others: &[Option<Address>]) -> Result<(), EngineError> {
    if others.iter().any(|other| other.as_ref() == Some(candidate)) {
        return Err(EngineError::InvalidAddress);
    }
    Ok(())
}

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
    assert_distinct(
        &admin,
        &[
            Some(vault.clone()),
            Some(pool.clone()),
            Some(oracle.clone()),
        ],
    )?;
    assert_distinct(&vault, &[Some(pool.clone()), Some(oracle.clone())])?;
    assert_distinct(&pool, &[Some(oracle.clone())])?;

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

    assert_distinct(&new_admin, &[vault, pool, oracle])?;

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

    assert_distinct(&vault, &[Some(admin.clone()), current_pool, current_oracle])?;

    admin.require_auth();

    storage::set_vault(&env, &vault);

    Ok(())
}

pub fn set_pool(env: Env, pool: Address) -> Result<(), EngineError> {
    let admin = storage::get_admin(&env).ok_or(EngineError::NotInitialized)?;

    // Validate pool doesn't collide with admin, vault, or oracle
    let current_vault = storage::get_vault(&env);
    let current_oracle = storage::get_oracle(&env);

    assert_distinct(&pool, &[Some(admin.clone()), current_vault, current_oracle])?;

    admin.require_auth();

    storage::set_pool(&env, &pool);

    Ok(())
}

pub fn set_oracle(env: Env, oracle: Address) -> Result<(), EngineError> {
    let admin = storage::get_admin(&env).ok_or(EngineError::NotInitialized)?;

    // Validate oracle doesn't collide with admin, vault, or pool
    let current_vault = storage::get_vault(&env);
    let current_pool = storage::get_pool(&env);

    assert_distinct(&oracle, &[Some(admin.clone()), current_vault, current_pool])?;

    admin.require_auth();

    storage::set_oracle(&env, &oracle);

    Ok(())
}
