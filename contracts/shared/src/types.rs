use soroban_sdk::{contracttype, Address, Vec};

use crate::constant::{BPS_DENOMINATOR, SECONDS_PER_YEAR};
use crate::errors::SharedError;

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PriceData {
    pub price: i128,
    pub timestamp: u64,
    pub write_timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Debt {
    pub principal: i128,
    pub accrued_interest: i128,
    pub interest_rate_bps: u32,
    pub last_accrual_at: u64,
}

impl Debt {
    pub fn total(&self) -> Result<i128, SharedError> {
        self.principal
            .checked_add(self.accrued_interest)
            .ok_or(SharedError::Overflow)
    }
}

/// Protocol-wide configuration for a supported asset.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct AssetConfig {
    pub token_decimals: u32,
    pub oracle_price_decimals: u32,
    pub max_ltv_bps: u32,
    pub liquidation_threshold_bps: u32,
}

pub const DEFAULT_TOKEN_DECIMALS: u32 = 7;
pub const DEFAULT_ORACLE_PRICE_DECIMALS: u32 = 7;

impl Default for AssetConfig {
    fn default() -> Self {
        Self {
            token_decimals: DEFAULT_TOKEN_DECIMALS,
            oracle_price_decimals: DEFAULT_ORACLE_PRICE_DECIMALS,
            max_ltv_bps: crate::DEFAULT_MAX_LTV_BPS,
            liquidation_threshold_bps: crate::DEFAULT_LIQUIDATION_THRESHOLD_BPS,
        }
    }
}

/// Represents a single collateral asset held by a user.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CollateralAsset {
    pub asset: Address,
    pub amount: i128,
}

/// Represents a user's collateral position across all assets.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    pub user: Address,
    pub collateral: Vec<CollateralAsset>,
}

pub fn accrue_linear_interest(
    principal: i128,
    rate_bps: u32,
    elapsed_seconds: u64,
) -> Result<i128, SharedError> {
    Ok(accrue_with_remainder(principal, rate_bps, elapsed_seconds, 0)?.0)
}

/// Carries fractional base units across accruals, including principal changes.
pub fn accrue_with_remainder(
    principal: i128,
    rate_bps: u32,
    elapsed_seconds: u64,
    remainder: i128,
) -> Result<(i128, i128), SharedError> {
    let denominator = BPS_DENOMINATOR * i128::from(SECONDS_PER_YEAR);
    if principal < 0 || !(0..denominator).contains(&remainder) {
        return Err(SharedError::InvalidAmount);
    }
    let rate = principal
        .checked_mul(i128::from(rate_bps))
        .ok_or(SharedError::Overflow)?;
    let elapsed = i128::from(elapsed_seconds);
    // Both factors in the fractional term are bounded by the denominator and u64.
    let fractional = (rate % denominator) * elapsed + remainder;
    let whole = (rate / denominator)
        .checked_mul(elapsed)
        .and_then(|amount| amount.checked_add(fractional / denominator))
        .ok_or(SharedError::Overflow)?;
    Ok((whole, fractional % denominator))
}

pub fn health_factor_bps(
    collateral_value: i128,
    debt: i128,
    liquidation_threshold_bps: u32,
) -> Result<i128, SharedError> {
    if collateral_value < 0 {
        return Err(SharedError::InvalidAmount);
    }
    if debt == 0 {
        return Ok(i128::MAX);
    }
    if debt < 0 {
        return Err(SharedError::InvalidAmount);
    }
    let numerator = collateral_value
        .checked_mul(liquidation_threshold_bps as i128)
        .ok_or(SharedError::Overflow)?;
    numerator
        .checked_div(debt)
        .ok_or(SharedError::DivisionByZero)
}

pub fn borrow_limit_from_collateral(
    collateral_value: i128,
    max_ltv_bps: u32,
) -> Result<i128, SharedError> {
    if collateral_value < 0 {
        return Err(SharedError::InvalidAmount);
    }
    collateral_value
        .checked_mul(max_ltv_bps as i128)
        .ok_or(SharedError::Overflow)?
        .checked_div(BPS_DENOMINATOR)
        .ok_or(SharedError::DivisionByZero)
}

pub fn ceil_div(numerator: i128, denominator: i128) -> Result<i128, SharedError> {
    if denominator == 0 {
        return Err(SharedError::DivisionByZero);
    }
    let quotient = numerator
        .checked_div(denominator)
        .ok_or(SharedError::Overflow)?;
    let remainder = numerator
        .checked_rem(denominator)
        .ok_or(SharedError::Overflow)?;
    if remainder != 0 && ((remainder > 0) == (denominator > 0)) {
        quotient.checked_add(1).ok_or(SharedError::Overflow)
    } else {
        Ok(quotient)
    }
}
