use crate::{
    constants::{
        BPS_DENOMINATOR, MAX_LIQ_BONUS_BPS, MAX_LIQ_THRESHOLD_BPS, MAX_LTV_BPS, MIN_LIQ_BONUS_BPS,
        MIN_LIQ_THRESHOLD_BPS, MIN_LTV_BPS, PRICE_PRECISION,
    },
    errors::VaultError,
    storage,
    types::{AssetRiskParams, DataKey},
};
use soroban_sdk::{Address, Env};

// ---------------------------------------------------------------------------
// Computation result — NOT stored, NOT contracttype
// ---------------------------------------------------------------------------

/// Snapshot of a user's position health at current oracle prices.
///
/// All values are denominated in USD using the same PRICE_PRECISION scaling
/// as the rest of the vault.
pub struct PositionHealth {
    /// Total raw USD value of all collateral at current prices.
    #[allow(dead_code)]
    pub total_collateral_value: i128,
    /// Weighted borrowing power:
    /// sum over assets of (asset_usd_value * ltv_bps / BPS_DENOMINATOR).
    #[allow(dead_code)]
    pub borrowing_power: i128,
    /// Weighted liquidation value:
    /// sum over assets of (asset_usd_value * liquidation_threshold_bps / BPS_DENOMINATOR).
    pub liquidation_value: i128,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Validate that a set of risk parameters is internally consistent and within
/// protocol bounds.  All rules are checked; the first violation encountered is
/// returned.
///
/// Rules:
/// 1. ltv_bps >= MIN_LTV_BPS
/// 2. ltv_bps <= MAX_LTV_BPS
/// 3. liquidation_threshold_bps >= MIN_LIQ_THRESHOLD_BPS
/// 4. liquidation_threshold_bps <= MAX_LIQ_THRESHOLD_BPS
/// 5. ltv_bps < liquidation_threshold_bps  (strictly less)
/// 6. liquidation_bonus_bps >= MIN_LIQ_BONUS_BPS
/// 7. liquidation_bonus_bps <= MAX_LIQ_BONUS_BPS
pub fn validate_risk_params(env: &Env, params: &AssetRiskParams) -> Result<(), VaultError> {
    if params.ltv_bps < MIN_LTV_BPS {
        soroban_sdk::panic_with_error!(env, VaultError::InvalidRiskParams);
    }
    if params.ltv_bps > MAX_LTV_BPS {
        soroban_sdk::panic_with_error!(env, VaultError::InvalidRiskParams);
    }
    if params.liquidation_threshold_bps < MIN_LIQ_THRESHOLD_BPS {
        soroban_sdk::panic_with_error!(env, VaultError::InvalidRiskParams);
    }
    if params.liquidation_threshold_bps > MAX_LIQ_THRESHOLD_BPS {
        soroban_sdk::panic_with_error!(env, VaultError::InvalidRiskParams);
    }
    // LTV must be strictly below the liquidation threshold.
    if params.ltv_bps >= params.liquidation_threshold_bps {
        soroban_sdk::panic_with_error!(env, VaultError::InvalidRiskParams);
    }
    if params.liquidation_bonus_bps < MIN_LIQ_BONUS_BPS {
        soroban_sdk::panic_with_error!(env, VaultError::InvalidRiskParams);
    }
    if params.liquidation_bonus_bps > MAX_LIQ_BONUS_BPS {
        soroban_sdk::panic_with_error!(env, VaultError::InvalidRiskParams);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Storage helpers
// ---------------------------------------------------------------------------

pub fn set_risk_params(env: &Env, asset: &Address, params: &AssetRiskParams) {
    env.storage()
        .persistent()
        .set(&DataKey::RiskParams(asset.clone()), params);
}

pub fn get_risk_params(env: &Env, asset: &Address) -> Option<AssetRiskParams> {
    env.storage()
        .persistent()
        .get(&DataKey::RiskParams(asset.clone()))
}

/// Returns the risk params for `asset`, panicking with `RiskParamsNotSet` when
/// none have been configured.
pub fn require_risk_params(env: &Env, asset: &Address) -> AssetRiskParams {
    match get_risk_params(env, asset) {
        Some(p) => p,
        None => soroban_sdk::panic_with_error!(env, VaultError::RiskParamsNotSet),
    }
}

// ---------------------------------------------------------------------------
// Health calculation
// ---------------------------------------------------------------------------

/// Compute the weighted health metrics for a user's current position.
///
/// For each collateral asset i held by the user:
///
///   asset_usd_value_i = balance_i * price_i / PRICE_PRECISION
///   borrowing_power_i  = asset_usd_value_i * ltv_bps_i / BPS_DENOMINATOR
///   liquidation_value_i = asset_usd_value_i * liq_threshold_bps_i / BPS_DENOMINATOR
///
/// Totals are the sum across all assets with non-zero balances.
///
/// # Panics
/// Panics with `RiskParamsNotSet` if any asset in the position has no risk
/// configuration, and with `NoPosition` if the user has no position at all.
pub fn compute_position_health(
    env: &Env,
    user: &Address,
    oracle_address: &Address,
) -> PositionHealth {
    use crate::OracleClient;

    let position = match storage::get_position(env, user) {
        Some(p) => p,
        None => soroban_sdk::panic_with_error!(env, VaultError::NoPosition),
    };

    let oracle_client = OracleClient::new(env, oracle_address);

    let mut total_collateral_value: i128 = 0;
    let mut borrowing_power: i128 = 0;
    let mut liquidation_value: i128 = 0;

    for item in position.collateral.iter() {
        let price_data = oracle_client.get_price_or_fail(&item.asset);
        let params = require_risk_params(env, &item.asset);

        // Raw USD value of this collateral leg.
        let asset_usd = item
            .amount
            .checked_mul(price_data.price)
            .unwrap_or_else(|| panic!("overflow computing asset USD value"))
            / PRICE_PRECISION;

        // Weighted borrowing power contribution.
        let bp_contrib = asset_usd
            .checked_mul(params.ltv_bps)
            .unwrap_or_else(|| panic!("overflow computing borrowing power"))
            / BPS_DENOMINATOR;

        // Weighted liquidation value contribution.
        let lv_contrib = asset_usd
            .checked_mul(params.liquidation_threshold_bps)
            .unwrap_or_else(|| panic!("overflow computing liquidation value"))
            / BPS_DENOMINATOR;

        total_collateral_value = total_collateral_value
            .checked_add(asset_usd)
            .unwrap_or_else(|| panic!("overflow summing collateral value"));

        borrowing_power = borrowing_power
            .checked_add(bp_contrib)
            .unwrap_or_else(|| panic!("overflow summing borrowing power"));

        liquidation_value = liquidation_value
            .checked_add(lv_contrib)
            .unwrap_or_else(|| panic!("overflow summing liquidation value"));
    }

    PositionHealth {
        total_collateral_value,
        borrowing_power,
        liquidation_value,
    }
}

// ---------------------------------------------------------------------------
// Withdrawal safety
// ---------------------------------------------------------------------------

/// Returns `true` when removing `amount` of `asset` from `user`'s position
/// leaves the weighted liquidation value >= outstanding `debt`.
///
/// - If `debt == 0`, always returns `true` (no constraint to enforce).
/// - If risk params for any asset are missing and debt > 0, panics with
///   `RiskParamsNotSet`.
/// - If the oracle price for `asset` is unavailable, panics (oracle contract
///   behaviour).
pub fn is_withdrawal_safe(
    env: &Env,
    user: &Address,
    asset: &Address,
    amount: i128,
    debt: i128,
    oracle_address: &Address,
) -> bool {
    if debt == 0 {
        return true;
    }

    use crate::OracleClient;

    // Compute the current weighted liquidation value of the full position.
    let health = compute_position_health(env, user, oracle_address);

    // Compute the liquidation-value contribution that would be removed.
    let oracle_client = OracleClient::new(env, oracle_address);
    let price_data = oracle_client
        .get_price(asset)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(env, VaultError::StalePrice));

    let params = require_risk_params(env, asset);

    let withdrawn_usd = amount
        .checked_mul(price_data.price)
        .unwrap_or_else(|| panic!("overflow in withdrawn USD calculation"))
        / PRICE_PRECISION;

    let withdrawn_lv = withdrawn_usd
        .checked_mul(params.liquidation_threshold_bps)
        .unwrap_or_else(|| panic!("overflow in withdrawn liquidation value"))
        / BPS_DENOMINATOR;

    // Guard: can't remove more liquidation value than currently exists.
    if health.liquidation_value < withdrawn_lv {
        return false;
    }

    let remaining_lv = health.liquidation_value - withdrawn_lv;

    // The position is safe as long as the remaining weighted liquidation value
    // covers the outstanding debt.
    remaining_lv >= debt
}
