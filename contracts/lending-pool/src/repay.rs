use shared::HEALTHY_HF_BPS;
use soroban_sdk::{token, Address, Env};

use crate::borrow::VaultClient;
use crate::debt;
use crate::errors::PoolError;
use crate::events;
use crate::storage;
use crate::types::PauseFlag;

/// Applies `amount` of `borrow_asset` to `user`'s debt, interest first, then
/// principal. Pulls `min(amount, total_debt)` from `user`.
pub fn repay(env: Env, user: Address, asset: Address, amount: i128) -> Result<(), PoolError> {
    user.require_auth();

    if amount <= 0 {
        return Err(PoolError::InvalidAmount);
    }

    let borrow_asset = storage::get_borrow_asset(&env).ok_or(PoolError::NotInitialized)?;
    if asset != borrow_asset {
        return Err(PoolError::UnsupportedAsset);
    }

    if storage::is_operation_paused(&env, &PauseFlag::Repay) {
        return Err(PoolError::RepayPaused);
    }

    let mut user_debt = debt::accrue_interest(env.clone(), user.clone())?;
    let total_debt = user_debt.total()?;

    if total_debt == 0 {
        return Err(PoolError::NoDebt);
    }

    let payment = amount.min(total_debt);
    let interest_paid = payment.min(user_debt.accrued_interest);
    let principal_paid = payment - interest_paid;

    let remaining = total_debt - payment;
    if remaining > 0 && remaining < shared::MIN_REMAINING_DEBT {
        return Err(PoolError::BelowMinDebt);
    }

    let token_client = token::Client::new(&env, &borrow_asset);
    token_client.transfer(&user, env.current_contract_address(), &payment);

    user_debt.accrued_interest = user_debt
        .accrued_interest
        .checked_sub(interest_paid)
        .ok_or(PoolError::Overflow)?;
    user_debt.principal = user_debt
        .principal
        .checked_sub(principal_paid)
        .ok_or(PoolError::Overflow)?;

    if remaining == 0 {
        storage::remove_debt(&env, &user);
    } else {
        debt::store_debt(&env, &user, &user_debt);
    }

    let new_total_borrowed = storage::get_total_borrowed(&env)
        .checked_sub(principal_paid)
        .ok_or(PoolError::Overflow)?;
    storage::set_total_borrowed(&env, new_total_borrowed);

    events::Repaid {
        user,
        asset,
        amount: payment,
        interest_paid,
        principal_paid,
    }
    .publish(&env);

    Ok(())
}

pub fn repay_for(
    env: Env,
    payer: Address,
    user: Address,
    asset: Address,
    amount: i128,
) -> Result<(), PoolError> {
    let _ = (env, payer, user, asset, amount);
    Err(PoolError::NotImplemented)
}

pub fn is_liquidatable(env: Env, user: Address) -> Result<bool, PoolError> {
    let user_debt = debt::accrue_interest(env.clone(), user.clone())?;
    let total_debt = user_debt.total()?;

    if total_debt == 0 {
        return Ok(false);
    }

    let vault_addr = storage::get_vault(&env).ok_or(PoolError::NotInitialized)?;
    let vault = VaultClient::new(&env, &vault_addr);

    Ok(vault.get_health_factor(&user) < HEALTHY_HF_BPS)
}
