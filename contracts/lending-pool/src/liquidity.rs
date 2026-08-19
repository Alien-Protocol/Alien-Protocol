use soroban_sdk::{token, Address, Env};

use crate::errors::PoolError;
use crate::events;
use crate::storage;
use crate::types::PauseFlag;

pub fn supply(env: Env, user: Address, amount: i128) -> Result<(), PoolError> {
    user.require_auth();

    if amount <= 0 {
        return Err(PoolError::InvalidAmount);
    }

    if storage::is_operation_paused(&env, &PauseFlag::Supply) {
        return Err(PoolError::PoolPaused);
    }

    let borrow_asset = storage::get_borrow_asset(&env).ok_or(PoolError::NotInitialized)?;

    let token_client = token::Client::new(&env, &borrow_asset);
    #[allow(clippy::needless_borrows_for_generic_args)]
    token_client.transfer(&user, env.current_contract_address(), &amount);

    let current_supply = storage::get_user_supply(&env, &user);
    let new_supply = current_supply
        .checked_add(amount)
        .ok_or(PoolError::Overflow)?;
    storage::set_user_supply(&env, &user, new_supply);

    let current_total = storage::get_total_supply(&env);
    let new_total = current_total
        .checked_add(amount)
        .ok_or(PoolError::Overflow)?;
    storage::set_total_supply(&env, new_total);

    events::Supplied { user, amount }.publish(&env);

    Ok(())
}

pub fn withdraw_liquidity(env: Env, user: Address, amount: i128) -> Result<(), PoolError> {
    user.require_auth();

    if amount <= 0 {
        return Err(PoolError::InvalidAmount);
    }

    if storage::is_operation_paused(&env, &PauseFlag::WithdrawLiquidity) {
        return Err(PoolError::PoolPaused);
    }

    let user_supply = storage::get_user_supply(&env, &user);
    if user_supply < amount {
        return Err(PoolError::InsufficientSupply);
    }

    let available = get_available_liquidity(env.clone());
    if available < amount {
        return Err(PoolError::InsufficientLiquidity);
    }

    let borrow_asset = storage::get_borrow_asset(&env).ok_or(PoolError::NotInitialized)?;

    let token_client = token::Client::new(&env, &borrow_asset);
    #[allow(clippy::needless_borrows_for_generic_args)]
    token_client.transfer(&env.current_contract_address(), &user, &amount);

    let new_supply = user_supply.checked_sub(amount).ok_or(PoolError::Overflow)?;
    storage::set_user_supply(&env, &user, new_supply);

    let current_total = storage::get_total_supply(&env);
    let new_total = current_total
        .checked_sub(amount)
        .ok_or(PoolError::Overflow)?;
    storage::set_total_supply(&env, new_total);

    events::LiquidityWithdrawn { user, amount }.publish(&env);

    Ok(())
}

pub fn get_user_supply(env: Env, user: Address) -> i128 {
    crate::storage::get_user_supply(&env, &user)
}

pub fn get_total_supply(env: Env) -> i128 {
    crate::storage::get_total_supply(&env)
}

pub fn get_available_liquidity(env: Env) -> i128 {
    let total_supply = storage::get_total_supply(&env);
    let total_borrowed = storage::get_total_borrowed(&env);
    total_supply.checked_sub(total_borrowed).unwrap_or(0)
}

pub fn get_utilization_bps(env: Env) -> u32 {
    let total_supply = storage::get_total_supply(&env);
    if total_supply == 0 {
        return 0;
    }
    let total_borrowed = storage::get_total_borrowed(&env);
    (total_borrowed * 10_000 / total_supply) as u32
}
