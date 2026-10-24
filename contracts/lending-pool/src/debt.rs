use soroban_sdk::{Address, Env};

use crate::errors::PoolError;
use crate::types::Debt;

pub fn load_debt(env: &Env, user: &Address) -> Debt {
    crate::storage::get_debt(env, user).unwrap_or(Debt {
        principal: 0,
        accrued_interest: 0,
        interest_rate_bps: 0,
        last_accrual_at: 0,
    })
}

pub fn store_debt(env: &Env, user: &Address, debt: &Debt) {
    crate::storage::set_debt(env, user, debt);
}

/// Accrues linear interest on `user`'s debt up to the current ledger
/// timestamp and persists the updated `accrued_interest`/`last_accrual_at`.
/// Principal is never touched here. A user with no debt yields a zero `Debt`
/// without writing to storage.
pub fn accrue_interest(env: Env, user: Address) -> Result<Debt, PoolError> {
    let mut debt = load_debt(&env, &user);

    if debt.principal == 0 && debt.accrued_interest == 0 {
        return Ok(debt);
    }

    let now = env.ledger().timestamp();
    let elapsed = now.saturating_sub(debt.last_accrual_at);

    if elapsed > 0 {
        let interest =
            shared::accrue_linear_interest(debt.principal, debt.interest_rate_bps, elapsed)?;
        debt.accrued_interest = debt
            .accrued_interest
            .checked_add(interest)
            .ok_or(PoolError::Overflow)?;
        debt.last_accrual_at = now;
        store_debt(&env, &user, &debt);
    }

    Ok(debt)
}

/// Returns `user`'s current debt after accruing any outstanding interest.
///
/// # Side effects
///
/// This is not a pure read: it calls `accrue_interest`, which *writes* the
/// updated `accrued_interest` and `last_accrual_at` values back to storage
/// whenever time has elapsed since the last accrual. Callers (and off-chain
/// simulations) invoking this function should expect a storage write / fee
/// when interest has accrued.
///
/// If no time has elapsed, or the user has no debt, no write occurs.
///
/// This accrue-on-read behavior is intentional so that reported debt is
/// always up to date. For a pure, no-write read (e.g. for off-chain
/// simulation), use `accrue_interest` on a cloned snapshot or compute from
/// the raw storage values via `crate::storage::get_debt`.
///
/// # Errors
///
/// Returns `PoolError::Overflow` if accrued interest overflows.
pub fn get_debt(env: Env, user: Address) -> Result<Debt, PoolError> {
    accrue_interest(env, user)
}

/// Returns the total debt (principal plus accrued interest) for `user`.
///
/// # Side effects
///
/// Like `get_debt`, this is not a pure read: it accrues interest and
/// *persists* the updated `accrued_interest` / `last_accrual_at` to storage
/// when time has elapsed since the last accrual. Callers (and off-chain
/// simulations) invoking this function should expect a storage write / fee
/// when interest has accrued.
///
/// If no time has elapsed, or the user has no debt, no write occurs.
///
/// # Errors
///
/// Returns `PoolError::Overflow` if accrued interest overflows.
pub fn get_user_debt(env: Env, user: Address) -> Result<i128, PoolError> {
    let debt = accrue_interest(env, user)?;
    Ok(debt.total()?)
}
