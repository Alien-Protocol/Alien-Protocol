use soroban_sdk::{Address, Env, Symbol};

use crate::errors::PoolError;
use crate::events;
use crate::storage::{self, get_pause_mask, set_pause_mask};
use crate::types::{pause_flag_symbol, PauseFlag};

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

    admin.require_auth();

    if vault == oracle {
        return Err(PoolError::InvalidAddress);
    }

    if interest_rate_bps == 0 {
        return Err(PoolError::InvalidRate);
    }

    storage::set_admin(&env, &admin);
    storage::set_vault(&env, &vault);
    storage::set_oracle(&env, &oracle);
    storage::set_borrow_asset(&env, &borrow_asset);
    storage::set_interest_rate_bps(&env, interest_rate_bps);
    storage::set_pause_mask(&env, 0);

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

    if current_admin == new_admin {
        return Err(PoolError::AlreadyAdmin);
    }

    let new_admin_clone = new_admin.clone();
    events::AdminChanged {
        old_admin: current_admin,
        new_admin: new_admin_clone,
    }
    .publish(&env);

    storage::set_admin(&env, &new_admin);
    Ok(())
}

pub fn set_vault(env: Env, vault: Address) -> Result<(), PoolError> {
    let admin = storage::get_admin(&env).ok_or(PoolError::NotInitialized)?;
    admin.require_auth();

    if let Some(current_oracle) = storage::get_oracle(&env) {
        if vault == current_oracle {
            return Err(PoolError::InvalidAddress);
        }
    }

    storage::set_vault(&env, &vault);
    Ok(())
}

pub fn set_oracle(env: Env, oracle: Address) -> Result<(), PoolError> {
    let admin = storage::get_admin(&env).ok_or(PoolError::NotInitialized)?;
    admin.require_auth();

    if let Some(current_vault) = storage::get_vault(&env) {
        if oracle == current_vault {
            return Err(PoolError::InvalidAddress);
        }
    }

    storage::set_oracle(&env, &oracle);
    Ok(())
}

pub fn pause_operation(env: Env, operation: PauseFlag, reason: Symbol) -> Result<(), PoolError> {
    let admin = storage::get_admin(&env).ok_or(PoolError::NotInitialized)?;
    admin.require_auth();

    let current_mask = get_pause_mask(&env);
    let flag_bit = operation.bit();

    if current_mask & flag_bit != 0 {
        return Err(PoolError::AlreadyPaused);
    }

    let new_mask = current_mask | flag_bit;
    set_pause_mask(&env, new_mask);

    let operation_symbol = pause_flag_symbol(&env, &operation);
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

    let current_mask = get_pause_mask(&env);
    let flag_bit = operation.bit();

    if current_mask & flag_bit == 0 {
        return Err(PoolError::NotPaused);
    }

    let new_mask = current_mask & !flag_bit;
    set_pause_mask(&env, new_mask);

    let operation_symbol = pause_flag_symbol(&env, &operation);
    events::OperationUnpaused {
        by: admin,
        operation: operation_symbol,
    }
    .publish(&env);

    Ok(())
}
