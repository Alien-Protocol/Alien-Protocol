use soroban_sdk::{Address, Env};

use crate::errors::PoolError;
use crate::types::Debt;

#[allow(dead_code)]
pub fn load_debt(env: &Env, user: &Address) -> Debt {
    crate::storage::get_debt(env, user).unwrap_or(Debt {
        principal: 0,
        accrued_interest: 0,
        interest_rate_bps: 0,
        last_accrual_at: 0,
    })
}

#[allow(dead_code)]
pub fn store_debt(env: &Env, user: &Address, debt: &Debt) {
    crate::storage::set_debt(env, user, debt);
}

pub fn accrue_interest(env: Env, user: Address) -> Result<Debt, PoolError> {
    let _ = (env, user);
    Err(PoolError::NotImplemented)
}

pub fn get_debt(env: Env, user: Address) -> Result<Debt, PoolError> {
    let _ = (env, user);
    Err(PoolError::NotImplemented)
}

pub fn get_user_debt(env: Env, user: Address) -> Result<i128, PoolError> {
    let _ = (env, user);
    Err(PoolError::NotImplemented)
}
