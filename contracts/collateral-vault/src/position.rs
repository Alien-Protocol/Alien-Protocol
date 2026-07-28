//! Position domain — deposit, withdraw, and shared debit/cleanup helpers.
//!
//! `debit_position` is the single implementation of "subtract amount from a
//! user's balance and clean up empty asset / user-index entries". Both
//! `withdraw` and `liquidation::seize_collateral` call it so the invariant
//! lives in exactly one place.

use crate::errors::VaultError;
use crate::events;
use crate::risk;
use crate::storage;
use soroban_sdk::{token, Address, Env};

// ─────────────────────────────────────────────────────────────────────────────
// Shared guards
// ─────────────────────────────────────────────────────────────────────────────

/// Panic if the vault is paused.
pub fn require_not_paused(env: &Env) {
    if storage::is_paused(env) {
        soroban_sdk::panic_with_error!(env, VaultError::VaultPaused);
    }
}

/// Panic if `asset` is not in the supported-asset index.
pub fn require_supported_asset(env: &Env, asset: &Address) {
    if !storage::is_supported_asset(env, asset) {
        soroban_sdk::panic_with_error!(env, VaultError::UnsupportedAsset);
    }
}

/// Panic if `user` has no active position.
pub fn require_position(env: &Env, user: &Address) {
    if storage::get_position(env, user).is_none() {
        soroban_sdk::panic_with_error!(env, VaultError::NoPosition);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared debit helper
// ─────────────────────────────────────────────────────────────────────────────

/// Subtract `amount` from `user`'s balance for `asset`, then clean up empty
/// asset / user-index entries.
///
/// Panics with `InvalidInputs` if `amount > balance`.
pub fn debit_position(env: &Env, user: &Address, asset: &Address, amount: i128) {
    let balance = storage::get_position_balance(env, user, asset);
    if amount > balance {
        soroban_sdk::panic_with_error!(env, VaultError::InvalidInputs);
    }

    let new_balance = balance - amount;
    storage::set_position_balance(env, user, asset, new_balance);

    if new_balance == 0 {
        storage::remove_user_asset(env, user, asset);
    }

    if storage::get_position(env, user).is_none() {
        storage::remove_from_position_index(env, user);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Write operations
// ─────────────────────────────────────────────────────────────────────────────

pub fn deposit(env: &Env, user: Address, asset: Address, amount: i128) {
    user.require_auth();

    if amount <= 0 {
        soroban_sdk::panic_with_error!(env, VaultError::InvalidInputs);
    }
    require_not_paused(env);
    require_supported_asset(env, &asset);

    let token_client = token::Client::new(env, &asset);
    token_client.transfer(&user, &env.current_contract_address(), &amount);

    let balance = storage::get_position_balance(env, &user, &asset);
    let new_balance = balance + amount;
    storage::set_position_balance(env, &user, &asset, new_balance);

    storage::add_user_asset(env, &user, &asset);
    storage::add_to_position_index(env, &user);

    events::Deposited {
        user,
        asset,
        amount,
    }
    .publish(env);
}

pub fn withdraw(env: &Env, user: Address, asset: Address, amount: i128) {
    user.require_auth();

    if amount <= 0 {
        soroban_sdk::panic_with_error!(env, VaultError::InvalidInputs);
    }
    require_not_paused(env);
    require_supported_asset(env, &asset);
    require_position(env, &user);

    let balance = storage::get_position_balance(env, &user, &asset);
    if amount > balance {
        soroban_sdk::panic_with_error!(env, VaultError::InvalidInputs);
    }

    // Safety check before any state mutation
    if !risk::is_withdrawal_safe(env, &user, &asset, amount) {
        soroban_sdk::panic_with_error!(env, VaultError::BelowMinCollateralRatio);
    }

    debit_position(env, &user, &asset, amount);

    let token_client = token::Client::new(env, &asset);
    token_client.transfer(&env.current_contract_address(), &user, &amount);

    events::Withdrawn {
        user,
        asset,
        amount,
    }
    .publish(env);
}
