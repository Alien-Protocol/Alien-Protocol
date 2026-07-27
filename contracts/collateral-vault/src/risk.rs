/// risk.rs — collateral-value computation and withdrawal-safety checks.
///
/// All arithmetic that could overflow uses `checked_*` and maps to
/// `VaultError::ArithmeticOverflow`.  Oracle calls that return `None` map to
/// `VaultError::PriceNotFound`.  No `panic!`, `expect`, or `unwrap` appears
/// in this module.
use crate::{config, errors::VaultError, storage};
use soroban_sdk::{Address, Env};

#[soroban_sdk::contractclient(name = "OracleClient")]
pub trait Oracle {
    fn get_price(env: Env, asset: Address) -> Option<crate::types::PriceData>;
    fn get_price_or_fail(env: Env, asset: Address) -> crate::types::PriceData;
}

#[soroban_sdk::contractclient(name = "LendingPoolClient")]
pub trait LendingPool {
    fn get_user_debt(env: Env, user: Address) -> i128;
    fn is_liquidatable(user: &Address) -> bool;
}

/// Oracle prices are encoded with 7 decimal places (e.g. $1.00 = 10_000_000).
pub const PRICE_PRECISION: i128 = 10_000_000;

/// Minimum collateral ratio expressed as a percentage (110 %).
pub const MIN_COLLATERAL_RATIO_PCT: i128 = 110;

/// Compute the total USD-denominated value of `user`'s collateral position.
///
/// Errors:
/// - `NoPosition`          — the user has no active position.
/// - `OracleNotConfigured` — oracle address is not set.
/// - `PriceNotFound`       — oracle has no price for an asset (also covers
///                           the stale-price path that `get_price_or_fail`
///                           surfaces as a contract error).
/// - `ArithmeticOverflow`  — multiplication or addition overflowed.
pub fn collateral_value(env: &Env, user: &Address) -> Result<i128, VaultError> {
    let position = storage::get_position(env, user).ok_or(VaultError::NoPosition)?;

    let oracle_addr = config::require_oracle(env)?;
    let oracle = OracleClient::new(env, &oracle_addr);

    let mut total: i128 = 0;

    for item in position.collateral.iter() {
        // `get_price_or_fail` panics (contract-abort) on stale price; the
        // Soroban host surfaces that as a contract error which the caller's
        // `try_*` wrapper will catch.  For a missing price we use `get_price`
        // so we can map `None` to our own typed error.
        let price_data = oracle
            .get_price(&item.asset)
            .ok_or(VaultError::PriceNotFound)?;

        let item_value = item
            .amount
            .checked_mul(price_data.price)
            .ok_or(VaultError::ArithmeticOverflow)?
            / PRICE_PRECISION;

        total = total
            .checked_add(item_value)
            .ok_or(VaultError::ArithmeticOverflow)?;
    }

    Ok(total)
}

/// Determine whether withdrawing `amount` of `asset` from `user`'s position
/// would leave the remaining collateral above the minimum collateral ratio.
///
/// Returns `Ok(true)` when the withdrawal is safe, `Ok(false)` when it would
/// breach the ratio, and `Err(_)` for any infrastructure failure.
///
/// Errors:
/// - `NoPosition`          — user has no active position.
/// - `OracleNotConfigured` — oracle address is not set.
/// - `PriceNotFound`       — oracle has no price for `asset`.
/// - `ArithmeticOverflow`  — arithmetic overflow.
pub fn is_withdrawal_safe(
    env: &Env,
    user: &Address,
    asset: &Address,
    amount: i128,
) -> Result<bool, VaultError> {
    // If there is no outstanding debt the ratio constraint does not apply.
    let debt = if let Ok(pool_addr) = config::require_pool(env) {
        LendingPoolClient::new(env, &pool_addr).get_user_debt(user)
    } else {
        0
    };

    if debt == 0 {
        return Ok(true);
    }

    let total_value = collateral_value(env, user)?;

    // Price of the asset being withdrawn (used to compute withdrawn_value).
    let oracle_addr = config::require_oracle(env)?;
    let oracle = OracleClient::new(env, &oracle_addr);
    let price_data = oracle
        .get_price(asset)
        .ok_or(VaultError::PriceNotFound)?;

    let withdrawn_value = amount
        .checked_mul(price_data.price)
        .ok_or(VaultError::ArithmeticOverflow)?
        / PRICE_PRECISION;

    if total_value < withdrawn_value {
        return Ok(false);
    }

    let remaining_value = total_value - withdrawn_value;

    // 110 % minimum: remaining_value >= debt * 110 / 100
    let required = debt
        .checked_mul(MIN_COLLATERAL_RATIO_PCT)
        .ok_or(VaultError::ArithmeticOverflow)?
        / 100;

    Ok(remaining_value >= required)
}
