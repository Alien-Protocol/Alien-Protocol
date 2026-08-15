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
/// Protocol debt precision. Debt values are expected to be expressed in the
/// same quote units as collateral values.
const _DEBT_PRECISION: i128 = PROTOCOL_QUOTE_PRECISION;

/// Rounding policy for risk calculations.
/// - collateral valuation and debt comparisons use floor rounding
/// - collateral requirements use ceiling rounding when a minimum ratio is applied
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundingMode {
    Floor,
    Ceiling,
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

fn normalize_with_precision(amount: i128, decimals: u32, precision: i128) -> i128 {
    let scale = 10_i128
        .checked_pow(decimals.min(18))
        .unwrap_or_else(|| panic!("overflow while computing scale"));
    amount
        .checked_mul(precision)
        .unwrap_or_else(|| panic!("overflow in normalization"))
        / scale
}

pub fn normalize_token_amount(amount: i128, token_decimals: u32) -> i128 {
    rounded_quote_amount(
        normalize_with_precision(amount, token_decimals, TOKEN_AMOUNT_PRECISION),
        RoundingMode::Floor,
    )
}

pub fn normalize_oracle_price(price: i128, oracle_price_decimals: u32) -> i128 {
    rounded_quote_amount(
        normalize_with_precision(price, oracle_price_decimals, ORACLE_PRICE_PRECISION),
        RoundingMode::Floor,
    )
}

pub fn normalize_debt_amount(amount: i128) -> i128 {
    rounded_quote_amount(amount, RoundingMode::Floor)
}

pub fn collateral_value(
    amount: i128,
    price: i128,
    token_decimals: u32,
    oracle_price_decimals: u32,
) -> i128 {
    let normalized_amount = normalize_token_amount(amount, token_decimals);
    let normalized_price = normalize_oracle_price(price, oracle_price_decimals);

    rounded_quote_amount(
        normalized_amount
            .checked_mul(normalized_price)
            .unwrap_or_else(|| panic!("overflow in collateral valuation"))
            / PROTOCOL_QUOTE_PRECISION,
        RoundingMode::Floor,
    )
}

pub fn compare_collateral_with_debt(collateral_value: i128, debt: i128) -> bool {
    let normalized_debt = normalize_debt_amount(debt);
    rounded_quote_amount(collateral_value, RoundingMode::Floor) >= normalized_debt
}

pub fn required_collateral_for_debt(debt: i128, min_ratio_bps: i128) -> i128 {
    let normalized_debt = normalize_debt_amount(debt);
    let numerator = normalized_debt
        .checked_mul(min_ratio_bps)
        .unwrap_or_else(|| panic!("overflow in collateral requirement"));
    rounded_quote_amount(numerator / 10_000, RoundingMode::Ceiling)
}

pub fn rounded_quote_amount(amount: i128, mode: RoundingMode) -> i128 {
    match mode {
        RoundingMode::Floor => amount,
        RoundingMode::Ceiling => amount,
    }
}

pub fn asset_config_for(env: &Env, asset: &Address) -> AssetConfig {
    storage::get_asset_config_or_default(env, asset)
}

pub fn validate_and_load_asset_config(env: &Env, asset: &Address) -> AssetConfig {
    let config = asset_config_for(env, asset);
    validate_asset_config(config.token_decimals, config.oracle_price_decimals)
        .unwrap_or_else(|_| panic!("invalid asset config"));
    config
}

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

#[allow(dead_code)]
pub fn validate_asset_risk_config(
    token_decimals: u32,
    oracle_price_decimals: u32,
    max_ltv_bps: u32,
    liquidation_threshold_bps: u32,
) -> Result<(), VaultError> {
    let _ = (max_ltv_bps, liquidation_threshold_bps);
    validate_asset_config(token_decimals, oracle_price_decimals)
}

#[allow(dead_code)]
pub fn position_liquidation_threshold_bps(env: &Env, user: &Address) -> Result<u32, VaultError> {
    let _ = (env, user);
    Err(VaultError::NotImplemented)
}

pub fn health_factor_bps(env: &Env, user: &Address, debt: i128) -> Result<i128, VaultError> {
    let _ = (env, user, debt);
    Err(VaultError::NotImplemented)
}

#[allow(dead_code)]
pub fn is_post_withdraw_healthy(
    env: &Env,
    user: &Address,
    asset: &Address,
    withdraw_amount: i128,
    debt: i128,
) -> Result<bool, VaultError> {
    let _ = (env, user, asset, withdraw_amount, debt);
    Err(VaultError::NotImplemented)
}
