use soroban_sdk::{token, Address, Env};

use crate::debt;
use crate::errors::PoolError;
use crate::events;
use crate::liquidity::get_available_liquidity;
use crate::storage;
use crate::types::PauseFlag;

#[soroban_sdk::contractclient(name = "VaultClient")]
#[allow(dead_code)]
pub trait Vault {
    fn get_collateral_value(env: Env, user: Address) -> i128;
    fn get_health_factor(env: Env, user: Address) -> i128;
    fn get_position(env: Env, user: Address) -> shared::Position;
    fn get_asset_config(env: Env, asset: Address) -> shared::AssetConfig;
}

pub fn borrow(env: Env, user: Address, asset: Address, amount: i128) -> Result<(), PoolError> {
    user.require_auth();

    if amount <= 0 {
        return Err(PoolError::InvalidAmount);
    }

    let borrow_asset = storage::get_borrow_asset(&env).ok_or(PoolError::NotInitialized)?;
    if asset != borrow_asset {
        return Err(PoolError::UnsupportedAsset);
    }

    if storage::is_operation_paused(&env, &PauseFlag::Borrow) {
        return Err(PoolError::BorrowPaused);
    }

    let mut user_debt = debt::accrue_interest(env.clone(), user.clone())?;

    if get_available_liquidity(env.clone()) < amount {
        return Err(PoolError::InsufficientLiquidity);
    }

    let limit = calculate_limit(env.clone(), user.clone())?;
    let current_total = user_debt.total()?;
    let new_debt = current_total
        .checked_add(amount)
        .ok_or(PoolError::Overflow)?;
    if new_debt > limit {
        return Err(PoolError::ExceedsBorrowLimit);
    }

    // First borrow establishes the debt's interest rate and accrual clock.
    if user_debt.principal == 0 && user_debt.accrued_interest == 0 {
        user_debt.interest_rate_bps = storage::get_interest_rate_bps(&env);
        user_debt.last_accrual_at = env.ledger().timestamp();
    }
    user_debt.principal = user_debt
        .principal
        .checked_add(amount)
        .ok_or(PoolError::Overflow)?;
    debt::store_debt(&env, &user, &user_debt);

    let new_total_borrowed = storage::get_total_borrowed(&env)
        .checked_add(amount)
        .ok_or(PoolError::Overflow)?;
    storage::set_total_borrowed(&env, new_total_borrowed);

    let token_client = token::Client::new(&env, &borrow_asset);
    token_client.transfer(&env.current_contract_address(), &user, &amount);

    events::Borrowed {
        user,
        asset,
        amount,
    }
    .publish(&env);

    Ok(())
}

/// Maximum total debt `user` may carry, computed as their vault collateral
/// value times the minimum max-LTV among their non-zero collateral assets.
/// Uses origination LTV, not the (higher) liquidation threshold.
pub fn calculate_limit(env: Env, user: Address) -> Result<i128, PoolError> {
    let vault_addr = storage::get_vault(&env).ok_or(PoolError::NotInitialized)?;
    let vault = VaultClient::new(&env, &vault_addr);

    let position = match vault.try_get_position(&user) {
        Ok(Ok(position)) => position,
        _ => return Err(PoolError::NoCollateral),
    };

    let mut max_ltv_bps: Option<u32> = None;
    for item in position.collateral.iter() {
        let config = vault.get_asset_config(&item.asset);
        max_ltv_bps = Some(match max_ltv_bps {
            Some(current) => current.min(config.max_ltv_bps),
            None => config.max_ltv_bps,
        });
    }
    let max_ltv_bps = max_ltv_bps.ok_or(PoolError::NoCollateral)?;

    let collateral_value = vault.get_collateral_value(&user);

    Ok(shared::borrow_limit_from_collateral(
        collateral_value,
        max_ltv_bps,
    )?)
}
