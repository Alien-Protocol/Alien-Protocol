use soroban_sdk::{Address, Env};
use crate::errors::VaultError;
use crate::storage;
use crate::types::Position;

/// Checks if a position is eligible for liquidation.
/// Queries current debt and asset prices to compute the collateral ratio dynamically.
pub fn verify_liquidation_eligibility(
    env: &Env,
    user: &Address,
    asset: &Address,
) -> Result<Position, VaultError> {
    // 1. Fetch user position
    let position = storage::get_position(env, user, asset)
        .ok_or(VaultError::NoPosition)?;

    if position.amount == 0 {
        return Err(VaultError::NoPosition);
    }

    // 2. Fetch configured oracle and lending pool addresses
    let oracle = storage::get_oracle(env)
        .ok_or(VaultError::OracleNotConfigured)?;
        
    let lending_pool = storage::get_lending_pool(env)
        .ok_or(VaultError::LendingPoolNotSet)?;

    // 3. Query collateral asset price from Oracle
    let asset_price = query_asset_price(env, &oracle, asset)?;
    if asset_price <= 0 {
        return Err(VaultError::PriceNotFound);
    }

    // 4. Query total user debt value in base currency from Lending Pool
    let total_debt = query_user_debt(env, &lending_pool, user)?;
    if total_debt == 0 {
        // User has no debt; position cannot be seized
        return Err(VaultError::PositionNotLiquidatable);
    }

    // 5. Calculate total collateral value = position.amount * asset_price
    let collateral_value = position
        .amount
        .checked_mul(asset_price)
        .ok_or(VaultError::MathOverflow)?;

    // 6. Calculate collateral ratio (in bps, e.g., 15000 = 150%)
    // Collateral Ratio = (Collateral Value * 10,000) / Total Debt
    let min_collateral_ratio = storage::get_min_collateral_ratio(env, asset)
        .ok_or(VaultError::UnsupportedAsset)?;

    let current_ratio = collateral_value
        .checked_mul(10_000)
        .ok_or(VaultError::MathOverflow)?
        .checked_div(total_debt)
        .ok_or(VaultError::MathOverflow)?;

    // 7. Position is liquidatable ONLY IF current_ratio < min_collateral_ratio
    if current_ratio >= min_collateral_ratio {
        return Err(VaultError::PositionNotLiquidatable);
    }

    Ok(position)
}

/// Helper to fetch asset price from external Oracle contract
fn query_asset_price(env: &Env, oracle: &Address, asset: &Address) -> Result<i128, VaultError> {
    // Call oracle contract client or cross-contract invocation
    env.invoke_contract(
        oracle,
        &soroban_sdk::Symbol::new(env, "get_price"),
        soroban_sdk::vec![env, asset.into_val(env)],
    )
}

/// Helper to fetch user debt from Lending Pool contract
fn query_user_debt(env: &Env, lending_pool: &Address, user: &Address) -> Result<i128, VaultError> {
    env.invoke_contract(
        lending_pool,
        &soroban_sdk::Symbol::new(env, "get_user_debt"),
        soroban_sdk::vec![env, user.into_val(env)],
    )
}
