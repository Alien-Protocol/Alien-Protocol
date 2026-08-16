use soroban_sdk::contracttype;

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

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct AssetRiskConfig {
    pub token_decimals: u32,
    pub oracle_price_decimals: u32,
    pub max_ltv_bps: u32,
    pub liquidation_threshold_bps: u32,
}

pub fn accrue_linear_interest(
    principal: i128,
    rate_bps: u32,
    elapsed_seconds: u64,
) -> Result<i128, SharedError> {
    if principal < 0 {
        return Err(SharedError::InvalidAmount);
    }
    let annual = principal
        .checked_mul(rate_bps as i128)
        .ok_or(SharedError::Overflow)?
        / BPS_DENOMINATOR;
    annual
        .checked_mul(elapsed_seconds as i128)
        .ok_or(SharedError::Overflow)?
        .checked_div(SECONDS_PER_YEAR as i128)
        .ok_or(SharedError::DivisionByZero)
}

pub fn health_factor_bps(
    collateral_value: i128,
    debt: i128,
    liquidation_threshold_bps: u32,
) -> Result<i128, SharedError> {
    if debt == 0 {
        return Ok(i128::MAX);
    }
    if debt < 0 {
        return Err(SharedError::InvalidAmount);
    }
    let numerator = collateral_value
        .checked_mul(liquidation_threshold_bps as i128)
        .ok_or(SharedError::Overflow)?;
    Ok(numerator / debt)
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
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder != 0 && ((remainder > 0) == (denominator > 0)) {
        quotient.checked_add(1).ok_or(SharedError::Overflow)
    } else {
        Ok(quotient)
    }
}
