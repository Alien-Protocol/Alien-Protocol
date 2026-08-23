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

    env.events().publish(
        ("engine", "initialized"),
        &Initialized {
            admin,
            vault,
            pool,
            oracle,
        },
    );

    Ok(())
}

pub fn set_admin(env: Env, new_admin: Address) -> Result<(), EngineError> {
    let current_admin = storage::get_admin(&env).ok_or(EngineError::NotInitialized)?;

    if current_admin == new_admin {
        return Err(EngineError::AlreadyAdmin);
    }

    current_admin.require_auth();

    storage::set_admin(&env, &new_admin);

    env.events().publish(
        ("engine", "admin_changed"),
        &AdminChanged {
            old_admin: current_admin,
            new_admin,
        },
    );

    Ok(())
}

pub fn set_vault(env: Env, vault: Address) -> Result<(), EngineError> {
    let admin = storage::get_admin(&env).ok_or(EngineError::NotInitialized)?;

    admin.require_auth();

    storage::set_vault(&env, &vault);

    Ok(())
}

pub fn set_pool(env: Env, pool: Address) -> Result<(), EngineError> {
    let admin = storage::get_admin(&env).ok_or(EngineError::NotInitialized)?;

    admin.require_auth();

    storage::set_pool(&env, &pool);

    Ok(())
}
