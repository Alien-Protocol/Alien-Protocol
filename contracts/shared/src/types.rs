use soroban_sdk::contracttype;

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
        let _ = self;
        Err(SharedError::NotImplemented)
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
    let _ = (principal, rate_bps, elapsed_seconds);
    Err(SharedError::NotImplemented)
}

pub fn health_factor_bps(
    collateral_value: i128,
    debt: i128,
    liquidation_threshold_bps: u32,
) -> Result<i128, SharedError> {
    let _ = (collateral_value, debt, liquidation_threshold_bps);
    Err(SharedError::NotImplemented)
}

pub fn borrow_limit_from_collateral(
    collateral_value: i128,
    max_ltv_bps: u32,
) -> Result<i128, SharedError> {
    let _ = (collateral_value, max_ltv_bps);
    Err(SharedError::NotImplemented)
}

pub fn ceil_div(numerator: i128, denominator: i128) -> Result<i128, SharedError> {
    let _ = (numerator, denominator);
    Err(SharedError::NotImplemented)
}
