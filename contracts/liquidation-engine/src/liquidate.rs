use soroban_sdk::{contracttype, token, Address, Env};

use crate::errors::EngineError;
use crate::events;
use crate::storage;
use crate::types::LiquidationResult;
use shared::{
    ceil_div, CollateralAsset, BPS_DENOMINATOR, CLOSE_FACTOR_BPS, HEALTHY_HF_BPS,
    LIQUIDATION_BONUS_BPS, MIN_REMAINING_DEBT, TARGET_HF_BPS,
};

/// Mirror of `collateral-vault`'s `PauseFlag`.
///
/// Unit-variant `contracttype` enums are encoded by variant name, so this
/// decodes identically on the vault side. Keep the variant names in sync with
/// `contracts/collateral-vault/src/types.rs`.
#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultPauseFlag {
    Deposit,
    Borrow,
    Withdraw,
    Liquidation,
    Recovery,
}

/// Mirror of `lending-pool`'s `PauseFlag`. Keep the variant names in sync with
/// `contracts/lending-pool/src/types.rs`.
#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoolPauseFlag {
    Supply,
    Borrow,
    Repay,
    WithdrawLiquidity,
}

#[soroban_sdk::contractclient(name = "VaultClient")]
#[allow(dead_code)]
pub trait Vault {
    fn seize_batch(
        env: Env,
        liquidation_engine: Address,
        user: Address,
        pool: Address,
        oracle: Address,
        basket: soroban_sdk::Vec<CollateralAsset>,
    );
    fn get_health_factor(env: Env, user: Address) -> i128;
    fn get_collateral_value(env: Env, user: Address) -> i128;
    fn get_position(env: Env, user: Address) -> shared::Position;
    fn get_asset_config(env: Env, asset: Address) -> shared::AssetConfig;
    fn get_liquidation_engine(env: Env) -> Option<Address>;
    fn is_operation_paused(env: Env, operation: VaultPauseFlag) -> bool;
}

#[soroban_sdk::contractclient(name = "PoolClient")]
#[allow(dead_code)]
pub trait Pool {
    fn get_user_debt(env: Env, user: Address) -> i128;
    fn repay_for(env: Env, payer: Address, user: Address, asset: Address, amount: i128);
    fn get_borrow_asset(env: Env) -> Option<Address>;
    fn is_operation_paused(env: Env, operation: PoolPauseFlag) -> bool;
}

#[soroban_sdk::contractclient(name = "OracleClient")]
#[allow(dead_code)]
pub trait Oracle {
    fn get_price_or_fail(env: Env, asset: Address) -> shared::types::PriceData;
}

/// Seize from the largest raw balance first. Equal balances follow position order.
pub fn liquidate(
    env: Env,
    liquidator: Address,
    user: Address,
    max_repay_amount: i128,
) -> Result<LiquidationResult, EngineError> {
    liquidator.require_auth();

    if liquidator == user {
        return Err(EngineError::Unauthorized);
    }

    if max_repay_amount <= 0 {
        return Err(EngineError::InvalidAmount);
    }

    let vault_addr = storage::get_vault(&env).ok_or(EngineError::NotInitialized)?;
    let pool_addr = storage::get_pool(&env).ok_or(EngineError::NotInitialized)?;
    let oracle_addr = storage::get_oracle(&env).ok_or(EngineError::NotInitialized)?;

    let vault = VaultClient::new(&env, &vault_addr);
    let pool = PoolClient::new(&env, &pool_addr);
    let oracle = OracleClient::new(&env, &oracle_addr);

    // Respect operator circuit breakers before touching any balances: a
    // liquidation seizes vault collateral and repays pool debt, so it must not
    // run while either of those paths is paused.
    if vault.is_operation_paused(&VaultPauseFlag::Liquidation) {
        return Err(EngineError::VaultPaused);
    }
    if pool.is_operation_paused(&PoolPauseFlag::Repay) {
        return Err(EngineError::PoolPaused);
    }

    let engine_address = env.current_contract_address();

    let registered_engine = match vault.try_get_liquidation_engine() {
        Ok(Ok(Some(engine))) => engine,
        _ => return Err(EngineError::Unauthorized),
    };
    if registered_engine != engine_address {
        return Err(EngineError::Unauthorized);
    }

    if !is_liquidatable(env.clone(), user.clone())? {
        return Err(EngineError::NotLiquidatable);
    }

    let max_partial_repay = calculate_partial_repayment(env.clone(), user.clone())?;
    let repay = max_repay_amount.min(max_partial_repay);

    let borrow_asset = match pool.try_get_borrow_asset() {
        Ok(Ok(Some(asset))) => asset,
        _ => return Err(EngineError::NotInitialized),
    };

    let bonus = calculate_bonus(env.clone(), repay)?;
    let seized_value = repay.checked_add(bonus).ok_or(EngineError::Overflow)?;
    let mut remaining_value = seized_value;

    // Seize collateral starting from the user's largest-balance asset (ties
    // broken by order in `get_position`), moving to the next asset whenever
    // one balance is insufficient to cover the remaining seized value.
    let position = match vault.try_get_position(&user) {
        Ok(Ok(pos)) => pos,
        _ => return Err(EngineError::NoPosition),
    };
    let mut collateral = position.collateral;
    let len = collateral.len();
    let mut basket = soroban_sdk::Vec::new(&env);

    for _ in 0..len {
        if remaining_value <= 0 {
            break;
        }

        let mut best_idx: Option<u32> = None;
        let mut best_amount: i128 = 0;
        for i in 0..len {
            let candidate = collateral.get_unchecked(i);
            if candidate.amount > best_amount {
                best_amount = candidate.amount;
                best_idx = Some(i);
            }
        }

        let idx = best_idx.ok_or(EngineError::InvalidAmount)?;
        let candidate = collateral.get_unchecked(idx);
        let asset = candidate.asset.clone();
        let balance = candidate.amount;

        let asset_config = match vault.try_get_asset_config(&asset) {
            Ok(Ok(config)) => config,
            _ => return Err(EngineError::NoPosition),
        };
        let price_data = match oracle.try_get_price_or_fail(&asset) {
            Ok(Ok(pd)) => pd,
            _ => return Err(EngineError::InvalidAmount),
        };

        let balance_value = shared::valuation::quote_value(
            &env,
            balance,
            price_data.price,
            asset_config.token_decimals,
            asset_config.oracle_price_decimals,
        )
        .map_err(|_| EngineError::Overflow)?;
        let seize_amount = if balance_value >= remaining_value {
            let token_amount_needed = shared::valuation::tokens_for_quote(
                &env,
                remaining_value,
                price_data.price,
                asset_config.token_decimals,
                asset_config.oracle_price_decimals,
            )
            .map_err(|_| EngineError::Overflow)?;
            // An indivisible token unit may be worth much more than the
            // entire quoted payment. Reject a ceiling conversion that would
            // take even one additional whole quote base unit from the user.
            let actual = shared::valuation::quote_value(
                &env,
                token_amount_needed,
                price_data.price,
                asset_config.token_decimals,
                asset_config.oracle_price_decimals,
            )
            .map_err(|_| EngineError::Overflow)?;
            if actual > remaining_value {
                return Err(EngineError::InvalidAmount);
            }
            remaining_value = 0;
            token_amount_needed
        } else {
            remaining_value -= balance_value;
            balance
        };

        collateral.set(
            idx,
            CollateralAsset {
                asset: asset.clone(),
                amount: 0,
            },
        );

        basket.push_back(CollateralAsset {
            asset,
            amount: seize_amount,
        });
    }
    if remaining_value > 0 {
        return Err(EngineError::InvalidAmount);
    }
    if !matches!(
        vault.try_seize_batch(&engine_address, &user, &pool_addr, &oracle_addr, &basket),
        Ok(Ok(()))
    ) {
        return Err(EngineError::InvalidAmount);
    }
    for item in basket.iter() {
        token::Client::new(&env, &item.asset).transfer(&engine_address, &liquidator, &item.amount);
    }

    if !matches!(
        pool.try_repay_for(&liquidator, &user, &borrow_asset, &repay),
        Ok(Ok(()))
    ) {
        return Err(EngineError::InvalidAmount);
    }

    events::Liquidated {
        user: user.clone(),
        liquidator: liquidator.clone(),
        repaid: repay,
        seized: seized_value,
        bonus,
    }
    .publish(&env);

    Ok(LiquidationResult {
        user,
        repaid: repay,
        seized: seized_value,
        bonus,
    })
}

pub fn is_liquidatable(env: Env, user: Address) -> Result<bool, EngineError> {
    let vault_addr = storage::get_vault(&env).ok_or(EngineError::NotInitialized)?;
    let pool_addr = storage::get_pool(&env).ok_or(EngineError::NotInitialized)?;

    let vault = VaultClient::new(&env, &vault_addr);
    let pool = PoolClient::new(&env, &pool_addr);

    let debt = match pool.try_get_user_debt(&user) {
        Ok(Ok(debt)) => debt,
        _ => return Err(EngineError::RiskUnavailable),
    };
    if debt == 0 {
        return Ok(false);
    }

    let hf = match vault.try_get_health_factor(&user) {
        Ok(Ok(hf)) => hf,
        _ => return Err(EngineError::RiskUnavailable),
    };
    Ok(hf < HEALTHY_HF_BPS)
}

pub fn calculate_bonus(env: Env, repaid_amount: i128) -> Result<i128, EngineError> {
    let _ = env;

    if repaid_amount <= 0 {
        return Err(EngineError::InvalidAmount);
    }

    // bonus = ceil_div(repaid_amount * LIQUIDATION_BONUS_BPS, BPS_DENOMINATOR)
    // Ceiling so the liquidator always receives at least the full 8% bonus
    // on non-exact amounts — this is intentional and favours the liquidator
    // by at most 1 unit, keeping the incentive intact.
    let bonus_numerator = repaid_amount
        .checked_mul(LIQUIDATION_BONUS_BPS as i128)
        .ok_or(EngineError::Overflow)?;

    ceil_div(bonus_numerator, BPS_DENOMINATOR).map_err(|_| EngineError::Overflow)
}

/// Solve for the target health factor using the position's minimum threshold.
/// The close factor normally caps repayment. Full settlement may clear an
/// unsafe or dust residual, subject to collateral coverage.
pub fn calculate_partial_repayment(env: Env, user: Address) -> Result<i128, EngineError> {
    let vault_addr = storage::get_vault(&env).ok_or(EngineError::NotInitialized)?;
    let pool_addr = storage::get_pool(&env).ok_or(EngineError::NotInitialized)?;

    let vault = VaultClient::new(&env, &vault_addr);
    let pool = PoolClient::new(&env, &pool_addr);

    let debt = match pool.try_get_user_debt(&user) {
        Ok(Ok(debt)) => debt,
        _ => return Err(EngineError::NoPosition),
    };

    if debt == 0 {
        return Err(EngineError::NoPosition);
    }

    let collateral_value = match vault.try_get_collateral_value(&user) {
        Ok(Ok(val)) => val,
        _ => return Err(EngineError::NoPosition),
    };
    let health_factor_bps = match vault.try_get_health_factor(&user) {
        Ok(Ok(hf)) => hf,
        _ => return Err(EngineError::NotLiquidatable),
    };

    // If HF_bps >= 10_000, position is healthy
    if health_factor_bps >= HEALTHY_HF_BPS {
        return Err(EngineError::NotLiquidatable);
    }

    // The largest payment whose principal plus rounded-up bonus fits the
    // collateral. Preserve the pool's minimum residual when full cover fails.
    let covered = collateral_value
        .checked_mul(BPS_DENOMINATOR)
        .ok_or(EngineError::Overflow)?
        / (BPS_DENOMINATOR + i128::from(LIQUIDATION_BONUS_BPS));
    if covered < debt {
        let repay = covered.min(debt - MIN_REMAINING_DEBT);
        return if repay > 0 {
            Ok(repay)
        } else {
            Err(EngineError::InvalidAmount)
        };
    }

    let position = match vault.try_get_position(&user) {
        Ok(Ok(pos)) => pos,
        _ => return Err(EngineError::NoPosition),
    };

    // Recover liquidation threshold as the minimum LT among non-zero assets.
    // Iterate through collateral assets and find the minimum liquidation_threshold_bps.
    let mut min_lt_bps: i128 = i128::MAX;
    for collateral_asset in position.collateral.iter() {
        if collateral_asset.amount > 0 {
            let asset_config = match vault.try_get_asset_config(&collateral_asset.asset) {
                Ok(Ok(cfg)) => cfg,
                _ => return Err(EngineError::NoPosition),
            };
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
                .ok_or(EngineError::Overflow)?,
        )
        .ok_or(EngineError::Overflow)?;

    // Denominator: TARGET_HF_BPS - (BPS_DENOMINATOR + LIQUIDATION_BONUS_BPS) * lt_bps / BPS_DENOMINATOR
    // First compute: (BPS_DENOMINATOR + LIQUIDATION_BONUS_BPS) * lt_bps / BPS_DENOMINATOR
    let temp = (BPS_DENOMINATOR + LIQUIDATION_BONUS_BPS as i128)
        .checked_mul(lt_bps)
        .ok_or(EngineError::Overflow)?;
    let denominator_sub = ceil_div(temp, BPS_DENOMINATOR).map_err(|_| EngineError::Overflow)?;

    let denominator = TARGET_HF_BPS
        .checked_sub(denominator_sub)
        .ok_or(EngineError::Overflow)?;

    if denominator <= 0 {
        return Ok(debt);
    }

    // Calculate R = ceil_div(numerator, denominator)
    let mut repay = ceil_div(numerator, denominator).map_err(|_| EngineError::Overflow)?;

    // Cap R at close_factor = debt * CLOSE_FACTOR_BPS / 10_000 (floor)
    let close_factor = debt
        .checked_mul(CLOSE_FACTOR_BPS as i128)
        .ok_or(EngineError::Overflow)?
        / BPS_DENOMINATOR;

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
        .checked_mul(BPS_DENOMINATOR + LIQUIDATION_BONUS_BPS as i128)
        .ok_or(EngineError::Overflow)?
        / BPS_DENOMINATOR;

    let remaining_collateral_value = collateral_value
        .checked_sub(seized_value)
        .ok_or(EngineError::Overflow)?;

    let remaining_debt = debt.checked_sub(repay).ok_or(EngineError::Overflow)?;

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
