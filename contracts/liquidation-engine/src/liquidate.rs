use soroban_sdk::{token, Address, Env};

use crate::errors::EngineError;
use crate::events;
use crate::storage;
use crate::types::LiquidationResult;
use shared::{
    ceil_div, CollateralAsset, CLOSE_FACTOR_BPS, HEALTHY_HF_BPS, LIQUIDATION_BONUS_BPS,
    MIN_REMAINING_DEBT, PROTOCOL_QUOTE_PRECISION, TARGET_HF_BPS,
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
    fn get_position(env: Env, user: Address) -> shared::Position;
    fn get_asset_config(env: Env, asset: Address) -> shared::AssetConfig;
    fn get_liquidation_engine(env: Env) -> Option<Address>;
}

#[soroban_sdk::contractclient(name = "PoolClient")]
#[allow(dead_code)]
pub trait Pool {
    fn get_user_debt(env: Env, user: Address) -> i128;
    fn is_liquidatable(env: Env, user: Address) -> bool;
    fn repay_for(env: Env, payer: Address, user: Address, asset: Address, amount: i128);
    fn get_borrow_asset(env: Env) -> Option<Address>;
}

/// Oracle client for the liquidation engine, matching `oracle-adapter`'s
/// three-field `PriceData` (price, timestamp, write_timestamp). Both the vault
/// and the engine now use this same shared `PriceData`, so cross-contract
/// reads of `get_price`/`get_price_or_fail` decode consistently.
#[soroban_sdk::contractclient(name = "OracleClient")]
#[allow(dead_code)]
pub trait Oracle {
    fn get_price(env: Env, asset: Address) -> Option<shared::types::PriceData>;
    fn get_price_or_fail(env: Env, asset: Address) -> shared::types::PriceData;
}

/// Converts a quote-unit value (`PROTOCOL_QUOTE_PRECISION`-scaled) into base
/// token units of an asset, given its decimals and oracle price. Rounds up so
/// the protocol never under-seizes.
fn value_to_token_amount(
    value: i128,
    price: i128,
    token_decimals: u32,
    oracle_price_decimals: u32,
) -> Result<i128, EngineError> {
    let exponent = token_decimals
        .checked_add(oracle_price_decimals)
        .ok_or(EngineError::Overflow)?;
    let scale = 10i128.checked_pow(exponent).ok_or(EngineError::Overflow)?;
    let numerator = value.checked_mul(scale).ok_or(EngineError::Overflow)?;
    let denominator = PROTOCOL_QUOTE_PRECISION
        .checked_mul(price)
        .ok_or(EngineError::Overflow)?;
    ceil_div(numerator, denominator).map_err(|_| EngineError::Overflow)
}

/// Converts base token units of an asset back into a quote-unit value, given
/// its decimals and oracle price. Rounds down (floor) so that when a full
/// collateral balance is seized, the value credited toward the seizure target
/// never exceeds what was actually taken.
fn token_amount_to_value(
    amount: i128,
    price: i128,
    token_decimals: u32,
    oracle_price_decimals: u32,
) -> Result<i128, EngineError> {
    let exponent = token_decimals
        .checked_add(oracle_price_decimals)
        .ok_or(EngineError::Overflow)?;
    let scale = 10i128.checked_pow(exponent).ok_or(EngineError::Overflow)?;
    let numerator = amount
        .checked_mul(PROTOCOL_QUOTE_PRECISION)
        .ok_or(EngineError::Overflow)?
        .checked_mul(price)
        .ok_or(EngineError::Overflow)?;
    numerator.checked_div(scale).ok_or(EngineError::Overflow)
}

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

    let engine_address = env.current_contract_address();

    let registered_engine = vault
        .get_liquidation_engine()
        .ok_or(EngineError::Unauthorized)?;
    if registered_engine != engine_address {
        return Err(EngineError::Unauthorized);
    }

    if !is_liquidatable(env.clone(), user.clone())? {
        return Err(EngineError::NotLiquidatable);
    }

    let max_partial_repay = calculate_partial_repayment(env.clone(), user.clone())?;
    let repay = max_repay_amount.min(max_partial_repay);

    let borrow_asset = pool.get_borrow_asset().ok_or(EngineError::NotInitialized)?;

    // Pull the repayment from the liquidator into the engine, then forward it
    // to the pool on the engine's own behalf.
    token::Client::new(&env, &borrow_asset).transfer(&liquidator, &engine_address, &repay);
    pool.repay_for(&engine_address, &user, &borrow_asset, &repay);

    let bonus = calculate_bonus(env.clone(), repay)?;
    let seized_value = repay.checked_add(bonus).ok_or(EngineError::Overflow)?;
    let mut remaining_value = seized_value;

    // Seize collateral starting from the user's largest-balance asset (ties
    // broken by order in `get_position`), moving to the next asset whenever
    // one balance is insufficient to cover the remaining seized value.
    let position = vault.get_position(&user);
    let mut collateral = position.collateral.clone();
    let len = collateral.len();

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

        let asset_config = vault.get_asset_config(&asset);
        let price_data = oracle.get_price_or_fail(&asset);

        let token_amount_needed = value_to_token_amount(
            remaining_value,
            price_data.price,
            asset_config.token_decimals,
            asset_config.oracle_price_decimals,
        )?;

        let seize_amount = if balance >= token_amount_needed {
            remaining_value = 0;
            token_amount_needed
        } else {
            let value_covered = token_amount_to_value(
                balance,
                price_data.price,
                asset_config.token_decimals,
                asset_config.oracle_price_decimals,
            )?;
            remaining_value = remaining_value
                .checked_sub(value_covered)
                .ok_or(EngineError::Overflow)?;
            balance
        };

        // Mark this asset processed so it is not selected again.
        collateral.set(
            idx,
            CollateralAsset {
                asset: asset.clone(),
                amount: 0,
            },
        );

        if seize_amount > 0 {
            vault.seize_collateral(&engine_address, &user, &asset, &seize_amount);
            token::Client::new(&env, &asset).transfer(&engine_address, &liquidator, &seize_amount);
        }
    }

    if remaining_value > 0 {
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

    let debt = pool.get_user_debt(&user);
    if debt == 0 {
        return Ok(false);
    }

    Ok(vault.get_health_factor(&user) < HEALTHY_HF_BPS)
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

    ceil_div(bonus_numerator, 10_000).map_err(|_| EngineError::Overflow)
}

pub fn calculate_partial_repayment(env: Env, user: Address) -> Result<i128, EngineError> {
    let vault_addr = storage::get_vault(&env).ok_or(EngineError::NotInitialized)?;
    let pool_addr = storage::get_pool(&env).ok_or(EngineError::NotInitialized)?;

    let vault = VaultClient::new(&env, &vault_addr);
    let pool = PoolClient::new(&env, &pool_addr);

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
    let position = vault.get_position(&user);

    // Recover liquidation threshold as the minimum LT among non-zero assets.
    // Iterate through collateral assets and find the minimum liquidation_threshold_bps.
    let mut min_lt_bps: i128 = i128::MAX;
    for collateral_asset in position.collateral.iter() {
        if collateral_asset.amount > 0 {
            let asset_config = vault.get_asset_config(&collateral_asset.asset);
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

    // Denominator: TARGET_HF_BPS - (10_000 + LIQUIDATION_BONUS_BPS) * lt_bps / 10_000
    // First compute: (10_000 + LIQUIDATION_BONUS_BPS) * lt_bps / 10_000
    let temp = (10_000i128 + LIQUIDATION_BONUS_BPS as i128)
        .checked_mul(lt_bps)
        .ok_or(EngineError::Overflow)?;
    let denominator_sub = ceil_div(temp, 10_000).map_err(|_| EngineError::Overflow)?;

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
