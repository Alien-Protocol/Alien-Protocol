use soroban_sdk::{Address, Env};

use crate::errors::EngineError;
use crate::types::LiquidationResult;

#[soroban_sdk::contractclient(name = "VaultClient")]
#[allow(dead_code)]
pub trait Vault {
    fn seize_collateral(
        env: Env,
        liquidation_engine: Address,
        user: Address,
        asset: Address,
        amount: i128,
    );
    fn get_health_factor(env: Env, user: Address) -> i128;
    fn get_collateral_value(env: Env, user: Address) -> i128;
}

#[soroban_sdk::contractclient(name = "PoolClient")]
#[allow(dead_code)]
pub trait Pool {
    fn get_user_debt(env: Env, user: Address) -> i128;
    fn is_liquidatable(env: Env, user: Address) -> bool;
    fn repay_for(env: Env, payer: Address, user: Address, asset: Address, amount: i128);
}

pub fn liquidate(
    env: Env,
    liquidator: Address,
    user: Address,
    max_repay_amount: i128,
) -> Result<LiquidationResult, EngineError> {
    let _ = (env, liquidator, user, max_repay_amount);
    Err(EngineError::NotImplemented)
}

pub fn is_liquidatable(env: Env, user: Address) -> Result<bool, EngineError> {
    let _ = (env, user);
    Err(EngineError::NotImplemented)
}

pub fn calculate_bonus(env: Env, repaid_amount: i128) -> Result<i128, EngineError> {
    let _ = (env, repaid_amount, shared::LIQUIDATION_BONUS_BPS);
    Err(EngineError::NotImplemented)
}

pub fn calculate_partial_repayment(env: Env, user: Address) -> Result<i128, EngineError> {
    let _ = (env, user, shared::CLOSE_FACTOR_BPS, shared::TARGET_HF_BPS);
    Err(EngineError::NotImplemented)
}
