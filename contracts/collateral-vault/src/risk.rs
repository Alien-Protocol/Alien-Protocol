//! Risk and valuation domain.
//!
//! All collateral-ratio maths lives here. Nothing in this module writes
//! to storage — every function is a pure read (plus external oracle/pool calls).

use crate::clients::{LendingPoolClient, OracleClient};
use crate::errors::VaultError;
use crate::storage;
use crate::types::Position;
use soroban_sdk::{Address, Env};

/// Oracle prices are encoded with 7 decimal places ($1.00 = 10_000_000).
const PRICE_PRECISION: i128 = 10_000_000;

/// Minimum collateral ratio: 110%.
const MIN_COLLATERAL_RATIO_PCT: i128 = 110;

// ─────────────────────────────────────────────────────────────────────────────
// Dependency loaders
// ─────────────────────────────────────────────────────────────────────────────

pub(crate) fn oracle_client(env: &Env) -> OracleClient<'_> {
    let addr = storage::get_oracle(env)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, VaultError::NotInitialized));
    OracleClient::new(env, &addr)
}

pub(crate) fn pool_client(env: &Env) -> Option<LendingPoolClient<'_>> {
    storage::get_lending_pool(env).map(|addr| LendingPoolClient::new(env, &addr))
}

// ─────────────────────────────────────────────────────────────────────────────
// Valuation
// ─────────────────────────────────────────────────────────────────────────────

/// Sum the USD value of every collateral asset in `position`.
pub(crate) fn collateral_value(env: &Env, position: &Position) -> i128 {
    let oracle = oracle_client(env);
    let mut total: i128 = 0;

    for item in position.collateral.iter() {
        let price_data = oracle.get_price_or_fail(&item.asset);
        let item_value = item
            .amount
            .checked_mul(price_data.price)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, VaultError::InvalidInputs))
            / PRICE_PRECISION;

        total = total
            .checked_add(item_value)
            .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, VaultError::InvalidInputs));
    }

    total
}

/// Total collateral value for `user`. Panics with `NoPosition` if the user has no position.
pub fn get_collateral_value(env: &Env, user: &Address) -> i128 {
    match storage::get_position(env, user) {
        Some(pos) => collateral_value(env, &pos),
        None => soroban_sdk::panic_with_error!(env, VaultError::NoPosition),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Safety check
// ─────────────────────────────────────────────────────────────────────────────

/// Returns `true` when withdrawing `amount` of `asset` keeps the user's
/// remaining collateral at or above the minimum collateral ratio.
pub fn is_withdrawal_safe(env: &Env, user: &Address, asset: &Address, amount: i128) -> bool {
    let debt = pool_client(env)
        .and_then(|c| c.try_get_user_debt(user).ok())
        .and_then(|r| r.ok())
        .unwrap_or(0);

    if debt == 0 {
        return true;
    }

    let total_value = get_collateral_value(env, user);

    let oracle = oracle_client(env);
    let price_data = oracle
        .get_price(asset)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, VaultError::NotInitialized));

    let withdrawn_value = amount
        .checked_mul(price_data.price)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, VaultError::InvalidInputs))
        / PRICE_PRECISION;

    if total_value < withdrawn_value {
        return false;
    }

    let remaining_value = total_value - withdrawn_value;
    remaining_value >= (debt * MIN_COLLATERAL_RATIO_PCT) / 100
}
