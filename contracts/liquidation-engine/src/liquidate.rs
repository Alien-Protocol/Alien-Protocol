use soroban_sdk::{Address, Env, IntoVal};

use crate::errors::EngineError;
use crate::storage;
use crate::types::LiquidationResult;
use shared::{
    ceil_div, CLOSE_FACTOR_BPS, HEALTHY_HF_BPS, LIQUIDATION_BONUS_BPS, MIN_REMAINING_DEBT,
    TARGET_HF_BPS,
};

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

/// Helper struct to access vault methods not in the Vault trait
struct VaultHelper {
    addr: Address,
}

impl VaultHelper {
    fn new(addr: Address) -> Self {
        Self { addr }
    }
    
    fn get_position(&self, env: &Env, user: &Address) -> Result<shared::Position, EngineError> {
        env.invoke_contract(
            &self.addr,
            &soroban_sdk::Symbol::new(env, "get_position"),
            soroban_sdk::vec![env, user.into_val(env)],
        )
    }
    
    fn get_asset_config(&self, env: &Env, asset: &Address) -> Result<shared::AssetConfig, EngineError> {
        env.invoke_contract(
            &self.addr,
            &soroban_sdk::Symbol::new(env, "get_asset_config"),
            soroban_sdk::vec![env, asset.into_val(env)],
        )
    }
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
    let _ = env;
    
    // Reject non-positive amounts
    if repaid_amount <= 0 {
        return Err(EngineError::InvalidAmount);
    }
    
    // Calculate bonus: ceil_div(repaid_amount * LIQUIDATION_BONUS_BPS, 10_000)
    // bonus = ceil_div(repaid_amount * bonus_bps, 10_000)
    let bonus_numerator = repaid_amount
        .checked_mul(LIQUIDATION_BONUS_BPS as i128)
        .ok_or(EngineError::Overflow)?;
    
    ceil_div(bonus_numerator, 10_000)
        .map_err(|_| EngineError::Overflow)
}

pub fn calculate_partial_repayment(env: Env, user: Address) -> Result<i128, EngineError> {
    let vault_addr = storage::get_vault(&env).ok_or(EngineError::NotInitialized)?;
    let pool_addr = storage::get_pool(&env).ok_or(EngineError::NotInitialized)?;
    
    let vault = VaultClient::new(&env, &vault_addr);
    let pool = PoolClient::new(&env, &pool_addr);
    let vault_helper = VaultHelper::new(vault_addr);
    
    // Load accrued debt via pool.get_user_debt
    let debt = pool.get_user_debt(&user);
    
    // Empty debt handling: return NoPosition if debt is 0
    if debt == 0 {
        return Err(EngineError::NoPosition);
    }
    
    // Load collateral_value and health_factor_bps from vault
    let collateral_value = vault.get_collateral_value(&user);
    let health_factor_bps = vault.get_health_factor(&user);
    
    // If HF_bps >= 10_000, position is healthy
    if health_factor_bps >= HEALTHY_HF_BPS {
        return Err(EngineError::NotLiquidatable);
    }
    
    // Get position to determine liquidation threshold
    let position = vault_helper.get_position(&env, &user)?;
    
    // Recover liquidation threshold as the minimum LT among non-zero assets.
    // Iterate through collateral assets and find the minimum liquidation_threshold_bps.
    let mut min_lt_bps: i128 = i128::MAX;
    for collateral_asset in position.collateral.iter() {
        if collateral_asset.amount > 0 {
            let asset_config = vault_helper.get_asset_config(&env, &collateral_asset.asset)?;
            let lt_bps = asset_config.liquidation_threshold_bps as i128;
            if lt_bps < min_lt_bps {
                min_lt_bps = lt_bps;
            }
        }
    }
    
    // If all assets have zero balance or no assets, this shouldn't happen for a liquidatable user
    if min_lt_bps == i128::MAX {
        return Err(EngineError::NoPosition);
    }
    
    let lt_bps = min_lt_bps;
    
    // Solve for repay R such that after seizing R * (1 + bonus) of collateral value,
    // post-HF equals TARGET_HF_BPS (1.10):
    // R = ceil_div(
    //       TARGET_HF_BPS * debt - collateral_value * lt_bps,
    //       TARGET_HF_BPS - (10_000 + LIQUIDATION_BONUS_BPS) * lt_bps / 10_000
    //     )
    
    // Numerator: TARGET_HF_BPS * debt - collateral_value * lt_bps
    let numerator = TARGET_HF_BPS
        .checked_mul(debt)
        .ok_or(EngineError::Overflow)?
        .checked_sub(
            collateral_value
                .checked_mul(lt_bps)
                .ok_or(EngineError::Overflow)?
        )
        .ok_or(EngineError::Overflow)?;
    
    // Denominator: TARGET_HF_BPS - (10_000 + LIQUIDATION_BONUS_BPS) * lt_bps / 10_000
    // First compute: (10_000 + LIQUIDATION_BONUS_BPS) * lt_bps / 10_000
    let temp = (10_000i128 + LIQUIDATION_BONUS_BPS as i128)
        .checked_mul(lt_bps)
        .ok_or(EngineError::Overflow)?;
    let denominator_sub = ceil_div(temp, 10_000)
        .map_err(|_| EngineError::Overflow)?;
    
    let denominator = TARGET_HF_BPS
        .checked_sub(denominator_sub)
        .ok_or(EngineError::Overflow)?;
    
    if denominator <= 0 {
        return Ok(debt);
    }
    
    // Calculate R = ceil_div(numerator, denominator)
    let mut repay = ceil_div(numerator, denominator)
        .map_err(|_| EngineError::Overflow)?;
    
    // Cap R at close_factor = debt * CLOSE_FACTOR_BPS / 10_000 (floor)
    let close_factor = debt
        .checked_mul(CLOSE_FACTOR_BPS as i128)
        .ok_or(EngineError::Overflow)?
        / 10_000;
    
    if repay > close_factor {
        repay = close_factor;
    }
    
    // Cap R at debt
    if repay > debt {
        repay = debt;
    }
    
    // Unsafe leftover fallback: if applying the capped R (including the 8% seize)
    // would leave HF_bps < 10_000, return full debt.
    let seized_value = repay
        .checked_mul(10_000i128 + LIQUIDATION_BONUS_BPS as i128)
        .ok_or(EngineError::Overflow)?
        / 10_000;
    
    let remaining_collateral_value = collateral_value
        .checked_sub(seized_value)
        .ok_or(EngineError::Overflow)?;
    
    let remaining_debt = debt
        .checked_sub(repay)
        .ok_or(EngineError::Overflow)?;
    
    // post-HF = remaining_collateral_value * lt_bps / remaining_debt
    let post_hf = if remaining_debt == 0 {
        i128::MAX
    } else {
        remaining_collateral_value
            .checked_mul(lt_bps)
            .ok_or(EngineError::Overflow)?
            / remaining_debt
    };
    
    if post_hf < HEALTHY_HF_BPS {
        return Ok(debt);
    }
    
    // Dust fallback: if debt - R > 0 and debt - R < MIN_REMAINING_DEBT, return full debt
    if remaining_debt > 0 && remaining_debt < MIN_REMAINING_DEBT {
        return Ok(debt);
    }
    
    Ok(repay)
}
