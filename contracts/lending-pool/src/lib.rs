#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, Symbol};

use errors::PoolError;
use types::{Debt, PauseFlag};

#[contract]
pub struct PoolContract;

#[contractimpl]
impl PoolContract {
    pub fn initialize(
        env: Env,
        admin: Address,
        vault: Address,
        oracle: Address,
        borrow_asset: Address,
        interest_rate_bps: u32,
    ) -> Result<(), PoolError> {
        admin::initialize(env, admin, vault, oracle, borrow_asset, interest_rate_bps)
    }

    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), PoolError> {
        admin::set_admin(env, new_admin)
    }

    pub fn set_vault(env: Env, vault: Address) -> Result<(), PoolError> {
        admin::set_vault(env, vault)
    }

    pub fn set_oracle(env: Env, oracle: Address) -> Result<(), PoolError> {
        admin::set_oracle(env, oracle)
    }

    pub fn pause_operation(
        env: Env,
        operation: PauseFlag,
        reason: Symbol,
    ) -> Result<(), PoolError> {
        admin::pause_operation(env, operation, reason)
    }

    pub fn unpause_operation(env: Env, operation: PauseFlag) -> Result<(), PoolError> {
        admin::unpause_operation(env, operation)
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        storage::get_admin(&env)
    }

    pub fn get_vault(env: Env) -> Option<Address> {
        storage::get_vault(&env)
    }

    pub fn get_oracle(env: Env) -> Option<Address> {
        storage::get_oracle(&env)
    }

    pub fn get_borrow_asset(env: Env) -> Option<Address> {
        storage::get_borrow_asset(&env)
    }

    pub fn get_interest_rate_bps(env: Env) -> u32 {
        storage::get_interest_rate_bps(&env)
    }

    pub fn supply(env: Env, user: Address, amount: i128) -> Result<(), PoolError> {
        liquidity::supply(env, user, amount)
    }

    pub fn withdraw_liquidity(env: Env, user: Address, amount: i128) -> Result<(), PoolError> {
        liquidity::withdraw_liquidity(env, user, amount)
    }

    pub fn get_user_supply(env: Env, user: Address) -> i128 {
        liquidity::get_user_supply(env, user)
    }

    pub fn get_total_supply(env: Env) -> i128 {
        liquidity::get_total_supply(env)
    }

    pub fn get_available_liquidity(env: Env) -> i128 {
        liquidity::get_available_liquidity(env)
    }

    pub fn get_utilization_bps(env: Env) -> u32 {
        liquidity::get_utilization_bps(env)
    }

    pub fn accrue_interest(env: Env, user: Address) -> Result<Debt, PoolError> {
        debt::accrue_interest(env, user)
    }

    pub fn get_debt(env: Env, user: Address) -> Result<Debt, PoolError> {
        debt::get_debt(env, user)
    }

    pub fn get_user_debt(env: Env, user: Address) -> i128 {
        match debt::get_user_debt(env.clone(), user) {
            Ok(total) => total,
            Err(e) => soroban_sdk::panic_with_error!(&env, e),
        }
    }

    pub fn borrow(env: Env, user: Address, asset: Address, amount: i128) -> Result<(), PoolError> {
        borrow::borrow(env, user, asset, amount)
    }

    pub fn calculate_limit(env: Env, user: Address) -> i128 {
        match borrow::calculate_limit(env.clone(), user) {
            Ok(limit) => limit,
            Err(e) => soroban_sdk::panic_with_error!(&env, e),
        }
    }

    pub fn repay(env: Env, user: Address, asset: Address, amount: i128) -> Result<(), PoolError> {
        repay::repay(env, user, asset, amount)
    }

    pub fn is_liquidatable(env: Env, user: Address) -> bool {
        match repay::is_liquidatable(env.clone(), user) {
            Ok(flag) => flag,
            Err(e) => soroban_sdk::panic_with_error!(&env, e),
        }
    }
}

mod admin;
mod borrow;
mod debt;
mod errors;
mod events;
mod liquidity;
mod repay;
mod storage;
#[cfg(test)]
mod tests;
mod types;
