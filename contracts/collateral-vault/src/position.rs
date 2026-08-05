use soroban_sdk::{Address, Env};

use crate::errors::VaultError;
use crate::storage;

/// Validates that `amount` is strictly positive.
///
/// Returns `Err(InvalidAmount)` for zero or negative values.
pub fn validate_positive_amount(amount: i128) -> Result<(), VaultError> {
    if amount <= 0 {
        return Err(VaultError::InvalidAmount);
    }
    Ok(())
}

/// Shared checked debit: validates amount, checks position membership via O(1)
/// lookup, checks sufficient balance, subtracts, and cleans up empty indices.
///
/// Returns the new balance after the debit.
///
/// # Errors
/// - `InvalidAmount` if `amount <= 0`
/// - `NoPosition` if the user has no slot in the position index
/// - `InsufficientCollateral` if `balance < amount`
pub fn checked_debit(
    env: &Env,
    user: &Address,
    asset: &Address,
    amount: i128,
) -> Result<i128, VaultError> {
    validate_positive_amount(amount)?;

    // O(1) slot-based membership check — no Vec scan
    if !storage::user_in_position_index(env, user) {
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
