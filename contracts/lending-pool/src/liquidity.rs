use soroban_sdk::{token, Address, Env, I256};

use crate::errors::PoolError;
use crate::events;
use crate::storage;
use crate::types::PauseFlag;

/// Proportional conversions use a wide intermediate. All inputs are
/// nonnegative and the result is bounded by the pool's assets or shares.
fn convert(env: &Env, value: i128, numerator: i128, denominator: i128, round_up: bool) -> i128 {
    let product = I256::from_i128(env, value).mul(&I256::from_i128(env, numerator));
    let divisor = I256::from_i128(env, denominator);
    let adjusted = if round_up {
        product.add(&I256::from_i128(env, denominator - 1))
    } else {
        product
    };
    adjusted
        .div(&divisor)
        .to_i128()
        .expect("share conversion overflow")
}

pub fn credit_interest(env: &Env, interest: i128) -> Result<(), PoolError> {
    let assets = storage::get_total_supply(env);
    if interest > 0 && assets > 0 {
        storage::set_total_shares(env, storage::total_shares(env));
        storage::set_total_supply(
            env,
            assets.checked_add(interest).ok_or(PoolError::Overflow)?,
        );
    }
    Ok(())
}

/// Supply `amount` of the borrow asset into the pool, increasing
/// `user`'s supply balance and the pool's total supply.
///
/// ## Authorization
///
/// Requires `user` authorization.
///
/// ## Transfers
///
/// Transfers `amount` of the borrow asset from `user` to the pool
/// contract.
///
/// ## Notes
///
/// Entry point for the supply side of the pool. Withdrawals, borrows,
/// and repayments are handled by `withdraw_liquidity` and the borrow
/// module respectively.
///
/// ## Errors
///
/// - `PoolError::InvalidAmount` if `amount` is not positive.
/// - `PoolError::PoolPaused` if supply is paused.
/// - `PoolError::NotInitialized` if the borrow asset is not configured.
/// - `PoolError::Overflow` if the supply balance or total supply
///   would overflow.
///
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
    token_client.transfer(&user, env.current_contract_address(), &amount);

    let current_total = storage::get_total_supply(&env);
    let shares = storage::total_shares(&env);
    let minted = if current_total == 0 {
        amount
    } else {
        convert(&env, amount, shares, current_total, false)
    };
    if minted == 0 {
        return Err(PoolError::InvalidAmount);
    }
    storage::set_total_shares(&env, shares.checked_add(minted).ok_or(PoolError::Overflow)?);
    let current_supply = storage::get_user_supply(&env, &user);
    let new_supply = current_supply
        .checked_add(minted)
        .ok_or(PoolError::Overflow)?;
    storage::set_user_supply(&env, &user, new_supply);

    let new_total = current_total
        .checked_add(amount)
        .ok_or(PoolError::Overflow)?;
    storage::set_total_supply(&env, new_total);

    events::Supplied { user, amount }.publish(&env);

    Ok(())
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

    let user_supply = get_user_supply(env.clone(), user.clone());
    if user_supply < amount {
        return Err(PoolError::InsufficientSupply);
    }

    let available = get_available_liquidity(env.clone());
    if available < amount {
        return Err(PoolError::InsufficientLiquidity);
    }

    let borrow_asset = storage::get_borrow_asset(&env).ok_or(PoolError::NotInitialized)?;

    let token_client = token::Client::new(&env, &borrow_asset);
    token_client.transfer(&env.current_contract_address(), &user, &amount);

    let current_total = storage::get_total_supply(&env);
    let shares = storage::total_shares(&env);
    let owned = storage::get_user_supply(&env, &user);
    let burned = if amount == user_supply {
        owned
    } else {
        convert(&env, amount, shares, current_total, true)
    };
    if burned == owned && amount < user_supply {
        return Err(PoolError::InvalidAmount);
    }
    storage::set_user_supply(&env, &user, owned - burned);
    storage::set_total_shares(&env, shares - burned);
    let new_total = current_total
        .checked_sub(amount)
        .ok_or(PoolError::Overflow)?;
    storage::set_total_supply(&env, new_total);

    events::LiquidityWithdrawn { user, amount }.publish(&env);

    Ok(())
}

pub fn get_user_supply(env: Env, user: Address) -> i128 {
    let owned = storage::get_user_supply(&env, &user);
    if owned == 0 {
        return 0;
    }
    convert(
        &env,
        owned,
        storage::get_total_supply(&env),
        storage::total_shares(&env),
        false,
    )
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

    // Use checked_mul to avoid overflow when total_borrowed is large.
    // On overflow, the pool is beyond 100% utilization, so clamp to 10_000 bps.
    // Then divide and clamp the final result to 10_000 so utilization is
    // always in [0, 10_000] regardless of the stored values.
    let numer = match total_borrowed.checked_mul(10_000) {
        Some(n) => n,
        None => return 10_000,
    };
    let raw = numer / total_supply;
    raw.min(10_000) as u32
}
