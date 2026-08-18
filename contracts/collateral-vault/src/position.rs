use soroban_sdk::{Address, Env};

use crate::errors::VaultError;
use crate::storage;

/// Validates that `amount` is strictly positive.
///
/// Returns `Err(InvalidAmount)` for zero, negative, or i128::MIN values.
pub fn validate_positive_amount(amount: i128) -> Result<(), VaultError> {
    if amount <= 0 {
        return Err(VaultError::InvalidAmount);
    }
    Ok(())
}

/// Shared checked debit that validates the amount, checks sufficient balance,
/// performs the subtraction, updates storage, and cleans up index/asset tracking.
///
/// Returns the new balance after the debit.
///
/// # Errors
/// - `InvalidAmount` if `amount <= 0`
/// - `InsufficientCollateral` if `balance < amount`
/// - `NoPosition` if the user has no position in the index
pub fn checked_debit(
    env: &Env,
    user: &Address,
    asset: &Address,
    amount: i128,
) -> Result<i128, VaultError> {
    validate_positive_amount(amount)?;

    let index = storage::get_position_index(env);
    if !index.contains(user) {
        return Err(VaultError::NoPosition);
    }

    let balance = storage::get_position_balance(env, user, asset);
    if balance < amount {
        return Err(VaultError::InsufficientCollateral);
    }

    let new_balance = balance
        .checked_sub(amount)
        .ok_or(VaultError::InsufficientCollateral)?;
    storage::set_position_balance(env, user, asset, new_balance);

    if new_balance == 0 {
        storage::remove_user_asset(env, user, asset);
    }

    if storage::get_position(env, user).is_none() {
        storage::remove_from_position_index(env, user);
    }

    Ok(new_balance)
}

/// Shared checked credit that validates the amount, performs the addition,
/// updates storage, and adds the asset and user to storage tracking/index.
///
/// Returns the new balance after the credit.
///
/// # Errors
/// - `InvalidAmount` if `amount <= 0` or on overflow
pub fn checked_credit(
    env: &Env,
    user: &Address,
    asset: &Address,
    amount: i128,
) -> Result<i128, VaultError> {
    validate_positive_amount(amount)?;

    let balance = storage::get_position_balance(env, user, asset);
    let new_balance = balance
        .checked_add(amount)
        .ok_or(VaultError::InvalidAmount)?;

    storage::set_position_balance(env, user, asset, new_balance);
    storage::add_user_asset(env, user, asset);
    storage::add_to_position_index(env, user);

    Ok(new_balance)
}
