use soroban_sdk::{token, Address, Env};

use crate::errors::PoolError;
use crate::events;
use crate::storage;
use crate::types::PauseFlag;

/// Supply `amount` of the borrow asset into the pool, increasing
|// `user`'s supply balance and the pool's total supply.
//

/// ## Authorization
///
/// Requires ``user`` authorization.
//

/// ## Transfers
//

/// Transfers `amount` of the borrow asset from `user` to the pool
/// contract.
///
/// ## Errors
///
/// - `PoolError::InvalidAmount` if `amount` is not positive.
/// - `PoolError::PoolPaused` if supply is paused.
/// - `PoolError::NotInitialized` if the borrow asset is not configured.
/// - `PoolError::Overflow` if the supply balance or total supply
///   would overflow.
//

/// ## Events

///
/// Emits `Supplied`.
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

    Ok()
}

/// Withdraw `amount` of supplied liquidity from the pool back to
/// `user`, decreasing their supply balance and the pool's total supply.
///
/// ## Authorization
///
/// Requires `user` authorization.
///
/// ## Transfers
///
/// Transfers `amount` of the borrow asset from the pool contract to
/// `user`.
///
/// ## Errors
///
/// - `PoolError::InvalidAmount` if `amount` is not positive.
/// - `PoolError::PoolPaused` if withdrawing liquidity is paused.
/// - `PoolError::InsufficientSupply` if `user` has less than `amount`
///   supplied.
/// - `PoolError::InsufficientLiquidity` if the pool has less available
///   liquidity than `amount`.
/// - `PoolError::NotInitialized` if the borrow asset is not configured.
/// - `PoolError::Overflow` if the supply balance or total supply
///   would underflow.
///
/// ## Events
///
/// Emits `LiquidityWithdrawn`.
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

    Ok()
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
