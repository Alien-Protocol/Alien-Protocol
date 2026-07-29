//! Configuration and administration domain.

use crate::{errors::VaultError, events, storage};
use soroban_sdk::{Address, Env};

pub fn initialize(
    env: &Env,
    admin: Address,
    lending_pool: Address,
    oracle: Address,
    liquidation_engine: Address,
) -> Result<(), VaultError> {
    if storage::has_admin(env) {
        return Err(VaultError::AlreadyInitialized);
    }

    if lending_pool == oracle {
        return Err(VaultError::InvalidAddress);
    }

    admin.require_auth();

    storage::set_admin(env, &admin);
    storage::set_lending_pool(env, &lending_pool);
    storage::set_oracle(env, &oracle);
    storage::set_liquidation_engine(env, &liquidation_engine);
    storage::set_paused(env, false);
    storage::set_contract_version(env, crate::upgrade::CURRENT_CONTRACT_VERSION);
    storage::set_storage_schema_version(env, crate::upgrade::CURRENT_STORAGE_SCHEMA_VERSION);

    events::Initialized {
        admin,
        lending_pool,
        oracle,
        liquidation_engine,
    }
    .publish(env);

    Ok(())
}

pub fn set_admin(env: Env, new_admin: Address) -> Result<(), VaultError> {
    let current_admin = storage::get_admin(&env).ok_or(VaultError::InvalidInputs)?;
    current_admin.require_auth();

    if current_admin == new_admin {
        return Err(VaultError::AlreadyAdmin);
    }

    storage::set_admin(&env, &new_admin);

    events::AdminChanged {
        old_admin: current_admin,
        new_admin,
    }
    .publish(&env);

    Ok(())
}

pub fn set_lending_pool(env: Env, lending_pool: Address) {
    let admin = storage::get_admin(&env)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
    admin.require_auth();

    if let Some(oracle) = storage::get_oracle(&env) {
        if lending_pool == oracle {
            soroban_sdk::panic_with_error!(&env, VaultError::InvalidAddress);
        }
    }

    let old_pool = storage::get_lending_pool(&env);
    storage::set_lending_pool(&env, &lending_pool);

    events::LendingPoolUpdated {
        old_pool,
        new_pool: lending_pool,
    }
    .publish(&env);
}

pub fn set_oracle(env: Env, oracle: Address) {
    let admin = storage::get_admin(&env)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
    admin.require_auth();

    if let Some(pool) = storage::get_lending_pool(&env) {
        if oracle == pool {
            soroban_sdk::panic_with_error!(&env, VaultError::InvalidAddress);
        }
    }

    let old_oracle = storage::get_oracle(&env);
    storage::set_oracle(&env, &oracle);

    events::OracleUpdated {
        old_oracle,
        new_oracle: oracle,
    }
    .publish(&env);
}

pub fn set_liquidation_engine(env: Env, engine: Address) {
    let admin = storage::get_admin(&env)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
    admin.require_auth();

    let old_engine = storage::get_liquidation_engine(&env);
    storage::set_liquidation_engine(&env, &engine);

    events::LiquidationEngineUpdated {
        old_engine,
        new_engine: engine,
    }
    .publish(&env);
}

pub fn pause(env: Env) {
    let admin = storage::get_admin(&env)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
    admin.require_auth();

    if storage::is_paused(&env) {
        soroban_sdk::panic_with_error!(&env, VaultError::AlreadyPaused);
    }

    storage::set_paused(&env, true);
    events::Paused { by: admin }.publish(&env);
}

pub fn unpause(env: Env) {
    let admin = storage::get_admin(&env)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
    admin.require_auth();

    if !storage::is_paused(&env) {
        soroban_sdk::panic_with_error!(&env, VaultError::NotPaused);
    }

    storage::set_paused(&env, false);
    events::Unpaused { by: admin }.publish(&env);
}
