use crate::{errors::VaultError, storage, types::AssetConfig};
use soroban_sdk::{Address, Env};

/// Protocol-wide fixed-point precision used for quote currency values.
/// Collateral, debt, and liquidation thresholds all share this precision so
/// they remain economically comparable regardless of token or oracle scaling.
pub const PROTOCOL_QUOTE_PRECISION: i128 = 10_000_000;
/// Standardized precision used when normalizing token base-unit balances.
pub const TOKEN_AMOUNT_PRECISION: i128 = PROTOCOL_QUOTE_PRECISION;
/// Standardized precision used when normalizing oracle prices.
pub const ORACLE_PRICE_PRECISION: i128 = PROTOCOL_QUOTE_PRECISION;

/// Rounding policy for risk calculations.
/// - collateral valuation and debt comparisons use floor rounding
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundingMode {
    Floor,
}

pub fn validate_asset_config(
    token_decimals: u32,
    oracle_price_decimals: u32,
) -> Result<(), VaultError> {
    if token_decimals == 0 || oracle_price_decimals == 0 {
        return Err(VaultError::InvalidAssetConfig);
    }

    if token_decimals > 18 || oracle_price_decimals > 18 {
        return Err(VaultError::InvalidAssetConfig);
    }

    Ok(())
}

fn normalize_with_precision(
    amount: i128,
    decimals: u32,
    precision: i128,
) -> Result<i128, VaultError> {
    let scale = 10_i128
        .checked_pow(decimals.min(18))
        .ok_or(VaultError::InvalidInputs)?;
    let scaled = amount
        .checked_mul(precision)
        .ok_or(VaultError::InvalidInputs)?;
    Ok(scaled / scale)
}

pub fn normalize_token_amount(amount: i128, token_decimals: u32) -> Result<i128, VaultError> {
    Ok(rounded_quote_amount(
        normalize_with_precision(amount, token_decimals, TOKEN_AMOUNT_PRECISION)?,
        RoundingMode::Floor,
    ))
}

pub fn normalize_oracle_price(price: i128, oracle_price_decimals: u32) -> Result<i128, VaultError> {
    Ok(rounded_quote_amount(
        normalize_with_precision(price, oracle_price_decimals, ORACLE_PRICE_PRECISION)?,
        RoundingMode::Floor,
    ))
}

/// Converts a token amount and oracle price into protocol quote precision.
///
/// `amount` and `price` are raw base-unit values whose decimal scales are
/// supplied by `token_decimals` and `oracle_price_decimals`. The result is
/// conservatively rounded down at each integer division.
///
/// # Panics
///
/// Panics when either decimal configuration cannot be normalized or when an
/// intermediate multiplication exceeds `i128`.
pub fn collateral_value(
    amount: i128,
    price: i128,
    token_decimals: u32,
    oracle_price_decimals: u32,
) -> i128 {
    let normalized_amount = normalize_token_amount(amount, token_decimals)
        .unwrap_or_else(|_| panic!("overflow in normalization"));
    let normalized_price = normalize_oracle_price(price, oracle_price_decimals)
        .unwrap_or_else(|_| panic!("overflow in normalization"));

    rounded_quote_amount(
        normalized_amount
            .checked_mul(normalized_price)
            .unwrap_or_else(|| panic!("overflow in collateral valuation"))
            / PROTOCOL_QUOTE_PRECISION,
        RoundingMode::Floor,
    )
}

pub fn rounded_quote_amount(amount: i128, mode: RoundingMode) -> i128 {
    match mode {
        RoundingMode::Floor => amount,
    }
}

pub fn asset_config_for(env: &Env, asset: &Address) -> AssetConfig {
    storage::get_asset_config_or_default(env, asset)
}

/// Loads the stored (or default) configuration for `asset` and validates its
/// token and oracle decimal scales before it is used in valuation.
///
/// # Panics
///
/// Panics when either decimal scale is zero or exceeds the supported 18-digit
/// precision bound.
pub fn validate_and_load_asset_config(env: &Env, asset: &Address) -> AssetConfig {
    let config = asset_config_for(env, asset);
    validate_asset_config(config.token_decimals, config.oracle_price_decimals)
        .unwrap_or_else(|_| panic!("invalid asset config"));
    config
}

/// Returns the sum of all of `user`'s collateral in protocol quote precision.
/// Each asset amount and oracle price is normalized independently and rounded
/// down, so the aggregate never overstates collateral because of division.
///
/// # Panics
///
/// Panics if the user has no position, the oracle is not configured, an asset
/// configuration or price cannot be loaded, or normalization/addition overflows.
pub fn normalized_collateral_value(env: &Env, user: &Address) -> i128 {
    let position = storage::get_position(env, user).unwrap_or_else(|| panic!("no position"));
    let mut total = 0_i128;
    let oracle_address = storage::get_oracle(env).expect("oracle not configured");
    let oracle_client = crate::OracleClient::new(env, &oracle_address);

    for item in position.collateral.iter() {
        let config = validate_and_load_asset_config(env, &item.asset);
        let price_data = oracle_client.get_price_or_fail(&item.asset);
        let item_value = collateral_value(
            item.amount,
            price_data.price,
            config.token_decimals,
            config.oracle_price_decimals,
        );
        total = total
            .checked_add(item_value)
            .unwrap_or_else(|| panic!("overflow in total value calculation"));
    }

    rounded_quote_amount(total, RoundingMode::Floor)
}

/// Validates decimal scales and basis-point risk limits for an asset.
///
/// Both LTV values must be in `1..=10_000`, and the liquidation threshold must
/// be strictly greater than maximum LTV. Returns `InvalidAssetConfig` for any
/// violation; this function does not panic.
pub fn validate_asset_risk_config(
    token_decimals: u32,
    oracle_price_decimals: u32,
    max_ltv_bps: u32,
    liquidation_threshold_bps: u32,
) -> Result<(), VaultError> {
    validate_asset_config(token_decimals, oracle_price_decimals)?;

    if !(1..=shared::BPS_DENOMINATOR as u32).contains(&max_ltv_bps)
        || !(1..=shared::BPS_DENOMINATOR as u32).contains(&liquidation_threshold_bps)
    {
        return Err(VaultError::InvalidAssetConfig);
    }
    if liquidation_threshold_bps <= max_ltv_bps {
        return Err(VaultError::InvalidAssetConfig);
    }

    Ok(())
}

/// Returns the conservative liquidation threshold for a position in basis
/// points: the minimum threshold among all non-zero collateral assets.
///
/// Returns `NoPosition` when the position is absent or contains no positive
/// collateral. This function performs no arithmetic that can overflow.
pub fn position_liquidation_threshold_bps(env: &Env, user: &Address) -> Result<u32, VaultError> {
    let position = storage::get_position(env, user).ok_or(VaultError::NoPosition)?;

    // Conservative v1 choice: a multi-asset position is only as safe as its
    // weakest collateral. Using the minimum liquidation threshold among the
    // user's non-zero assets avoids treating higher-LT assets as if they can
    // support lower-LT ones until a weighted LT is implemented.
    let mut min_lt: Option<u32> = None;
    for item in position.collateral.iter() {
        if item.amount == 0 {
            continue;
        }
        let config = asset_config_for(env, &item.asset);
        min_lt = Some(match min_lt {
            Some(current) => current.min(config.liquidation_threshold_bps),
            None => config.liquidation_threshold_bps,
        });
    }

    min_lt.ok_or(VaultError::NoPosition)
}

/// Computes `user`'s liquidation health factor in basis points from normalized
/// collateral, debt, and the position's conservative liquidation threshold.
/// Collateral valuation rounds down before the shared health-factor calculation.
///
/// Returns a `VaultError` for missing positions, invalid debt/configuration,
/// division by zero, or overflow. Valuation may panic under the conditions
/// documented by [`normalized_collateral_value`].
pub fn health_factor_bps(env: &Env, user: &Address, debt: i128) -> Result<i128, VaultError> {
    if storage::get_position(env, user).is_none() {
        return Err(VaultError::NoPosition);
    }

    let collateral_value = normalized_collateral_value(env, user);
    let liquidation_threshold_bps = position_liquidation_threshold_bps(env, user)?;
    shared::health_factor_bps(collateral_value, debt, liquidation_threshold_bps).map_err(|err| {
        match err {
            shared::SharedError::InvalidAmount => VaultError::InvalidAmount,
            shared::SharedError::Overflow => VaultError::InvalidInputs,
            shared::SharedError::InvalidBps => VaultError::InvalidAssetConfig,
            shared::SharedError::NotImplemented => VaultError::NotImplemented,
            shared::SharedError::DivisionByZero => VaultError::InvalidInputs,
        }
    })
}

/// Simulates withdrawing `withdraw_amount` of `asset` and reports whether the
/// remaining position has a health factor of at least 10,000 basis points.
/// Inputs are raw token/debt amounts; collateral values are normalized and
/// rounded down. Zero debt is always healthy.
///
/// Returns typed errors for non-positive withdrawals, missing positions,
/// insufficient balance, invalid shared-risk inputs, and checked subtraction.
///
/// # Panics
///
/// Panics when the oracle is unavailable, an asset configuration/price cannot
/// be loaded, or normalized collateral aggregation overflows.
pub fn is_post_withdraw_healthy(
    env: &Env,
    user: &Address,
    asset: &Address,
    withdraw_amount: i128,
    debt: i128,
) -> Result<bool, VaultError> {
    if debt <= 0 {
        return Ok(true);
    }

    if withdraw_amount <= 0 {
        return Err(VaultError::InvalidAmount);
    }

    let position = storage::get_position(env, user).ok_or(VaultError::NoPosition)?;
    let balance = storage::get_position_balance(env, user, asset);
    if balance < withdraw_amount {
        return Err(VaultError::InsufficientCollateral);
    }

    let mut total_value = 0_i128;
    let mut min_lt: Option<u32> = None;
    let oracle_address = storage::get_oracle(env).expect("oracle not configured");
    let oracle_client = crate::OracleClient::new(env, &oracle_address);

    for item in position.collateral.iter() {
        let post_amount = if item.asset == *asset {
            item.amount
                .checked_sub(withdraw_amount)
                .ok_or(VaultError::InsufficientCollateral)?
        } else {
            item.amount
        };

        if post_amount > 0 {
            let config = validate_and_load_asset_config(env, &item.asset);
            let price_data = oracle_client.get_price_or_fail(&item.asset);
            let item_value = collateral_value(
                post_amount,
                price_data.price,
                config.token_decimals,
                config.oracle_price_decimals,
            );
            total_value = total_value
                .checked_add(item_value)
                .unwrap_or_else(|| panic!("overflow in total value calculation"));

            min_lt = Some(match min_lt {
                Some(current) => current.min(config.liquidation_threshold_bps),
                None => config.liquidation_threshold_bps,
            });
        }
    }

    let lt_bps = match min_lt {
        Some(lt) => lt,
        None => return Ok(false),
    };

    let post_collateral_value = rounded_quote_amount(total_value, RoundingMode::Floor);
    let hf_bps =
        shared::health_factor_bps(post_collateral_value, debt, lt_bps).map_err(
            |err| match err {
                shared::SharedError::InvalidAmount => VaultError::InvalidAmount,
                shared::SharedError::Overflow => VaultError::InvalidInputs,
                shared::SharedError::InvalidBps => VaultError::InvalidAssetConfig,
                shared::SharedError::NotImplemented => VaultError::NotImplemented,
                shared::SharedError::DivisionByZero => VaultError::InvalidInputs,
            },
        )?;

    Ok(hf_bps >= shared::BPS_DENOMINATOR)
}

#[cfg(test)]
mod normalization_tests {
    use super::*;

    // Note: these tests exercise the pure normalization helpers only and do not
    // require a Soroban `Env`. Intended to live alongside `test_risk.rs`.

    #[test]
    fn token_amount_7_decimals_is_identity_at_protocol_precision() {
        // 7-decimal token, 7-decimal quote precision: 1.0000000 token == 10_000_000 base units
        assert_eq!(normalize_token_amount(10_000_000, 7), Ok(10_000_000));
        assert_eq!(normalize_token_amount(1, 7), Ok(1));
    }

    #[test]
    fn oracle_price_7_decimals_is_identity_at_protocol_precision() {
        assert_eq!(normalize_oracle_price(12_345_678, 7), Ok(12_345_678));
    }

    #[test]
    fn token_amount_18_decimal_boundary_floors() {
        // 18-decimal token: 10^18 base units == 1 token == 10^7 quote units.
        assert_eq!(normalize_token_amount(1_000_000_000_000_000_000, 18), Ok(10_000_000));
        // Sub-quote-unit remainder is floored away.
        assert_eq!(normalize_token_amount(999_999_999_999, 18), Ok(0));
    }

    #[test]
    fn oracle_price_18_decimal_boundary_floors() {
        assert_eq!(normalize_oracle_price(1_000_000_000_000_000_000, 18), Ok(10_000_000));
        assert_eq!(normalize_oracle_price(1_999_999_999_999, 18), Ok(19_999_999 / 1_000_000 * 0 + 0));
    }

    #[test]
    fn zero_amount_returns_zero() {
        assert_eq!(normalize_token_amount(0, 7), Ok(0));
        assert_eq!(normalize_token_amount(0, 18), Ok(0));
        assert_eq!(normalize_oracle_price(0, 7), Ok(0));
    }

    #[test]
    fn near_overflow_amount_returns_error_not_panic() {
        // amount * PROTOCOL_QUOTE_PRECISION overflows i128.
        assert_eq!(
            normalize_token_amount(i128::MAX, 7),
            Err(VaultError::InvalidInputs)
        );
        assert_eq!(
            normalize_oracle_price(i128::MAX, 7),
            Err(VaultError::InvalidInputs)
        );
    }

    #[test]
    fn rounding_is_floor_for_positive_and_truncation_toward_zero_for_fraction() {
        // 15 base units at 1 decimal => 15 * 10^7 / 10 = 15_000_000 (exact)
        assert_eq!(normalize_token_amount(15, 1), Ok(15_000_000));
        // 1 base unit at 8 decimals => 10^7 / 10^8 = 0.1 -> floor to 0
        assert_eq!(normalize_token_amount(1, 8), Ok(0));
        // 19 base units at 1 decimal => 190_000_000 / 10 = 19_000_000 exact; choose non-exact:
        // 3 base units at 2 decimals => 30_000_000 / 100 = 300_000 exact; 7 at 3 => 70_000_000/1000 = 70_000
        // Non-exact: 1 base unit at 3 decimals => 10_000_000 / 1000 = 10_000 exact.
        // Non-exact: 5 base units at 9 decimals => 50_000_000 / 10^9 = 0.05 -> 0 (floor).
        assert_eq!(normalize_token_amount(5, 9), Ok(0));
    }

    #[test]
    fn debt_normalization_is_protocol_precision_identity_at_seven_decimals() {
        // Debt is denominated directly in protocol quote precision; the same
        // helper path (token-amount normalization with 7 decimals) must be a
        // lossless identity for in-range values.
        assert_eq!(normalize_token_amount(250_000_000, 7), Ok(250_000_000));
        assert_eq!(normalize_token_amount(1, 7), Ok(1));
    }

    #[test]
    fn debt_normalization_near_max_reports_overflow() {
        assert_eq!(
            normalize_token_amount(i128::MAX / 2, 7),
            Err(VaultError::InvalidInputs)
        );
    }
}
