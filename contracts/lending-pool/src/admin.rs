use soroban_sdk::{Address, Env, Symbol};

use crate::errors::PoolError;
use crate::events;
use crate::storage;
use crate::types::PauseFlag;

pub fn initialize(
    env: Env,
    admin: Address,
    vault: Address,
    oracle: Address,
    borrow_asset: Address,
    interest_rate_bps: u32,
) -> Result<(), PoolError> {
    if storage::has_admin(&env) {
        return Err(PoolError::AlreadyInitialized);
    }

    storage::set_admin(&env, &admin);
    storage::set_vault(&env, &vault);
    storage::set_oracle(&env, &oracle);
    storage::set_borrow_asset(&env, &borrow_asset);
    storage::set_interest_rate_bps(&env, interest_rate_bps);

    events::Initialized {
        admin,
        vault,
        oracle,
        borrow_asset,
        interest_rate_bps,
    }
    .publish(&env);

    Ok(())
}

pub fn set_admin(env: Env, new_admin: Address) -> Result<(), PoolError> {
    let current_admin = storage::get_admin(&env).ok_or(PoolError::NotInitialized)?;
    current_admin.require_auth();

    storage::set_admin(&env, &new_admin);
    Ok(())
}

pub fn set_vault(env: Env, vault: Address) -> Result<(), PoolError> {
    let admin = storage::get_admin(&env).ok_or(PoolError::NotInitialized)?;
    admin.require_auth();

    storage::set_vault(&env, &vault);
    Ok(())
}

pub fn set_oracle(env: Env, oracle: Address) -> Result<(), PoolError> {
    let admin = storage::get_admin(&env).ok_or(PoolError::NotInitialized)?;
    admin.require_auth();

    storage::set_oracle(&env, &oracle);
    Ok(())
}

pub fn pause_operation(env: Env, operation: PauseFlag, reason: Symbol) -> Result<(), PoolError> {
    let admin = storage::get_admin(&env).ok_or(PoolError::NotInitialized)?;
    admin.require_auth();

    let current_mask = storage::get_pause_mask(&env);
    if current_mask & operation.bit() != 0 {
        return Err(PoolError::AlreadyPaused);
    }

    let new_mask = current_mask | operation.bit();
    storage::set_pause_mask(&env, new_mask);

    let operation_symbol = crate::types::pause_flag_symbol(&env, &operation);
    events::OperationPaused {
        by: admin,
        operation: operation_symbol,
        reason,
    }
    .publish(&env);

    Ok(())
}

pub fn unpause_operation(env: Env, operation: PauseFlag) -> Result<(), PoolError> {
    let admin = storage::get_admin(&env).ok_or(PoolError::NotInitialized)?;
    admin.require_auth();

    let current_mask = storage::get_pause_mask(&env);
    if current_mask & operation.bit() == 0 {
        return Err(PoolError::NotPaused);
    }

    let new_mask = current_mask & !operation.bit();
    storage::set_pause_mask(&env, new_mask);

    let operation_symbol = crate::types::pause_flag_symbol(&env, &operation);
    events::OperationUnpaused {
        by: admin,
        operation: operation_symbol,
    }
    .publish(&env);

    Ok(())
}
