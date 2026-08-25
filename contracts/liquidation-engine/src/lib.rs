#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env};

use errors::EngineError;
use types::LiquidationResult;

#[contract]
pub struct LiquidationContract;

#[contractimpl]
impl LiquidationContract {
    pub fn initialize(
        env: Env,
        admin: Address,
        vault: Address,
        pool: Address,
        oracle: Address,
    ) -> Result<(), EngineError> {
        admin::initialize(env, admin, vault, pool, oracle)
    }

    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), EngineError> {
        admin::set_admin(env, new_admin)
    }

    pub fn set_vault(env: Env, vault: Address) -> Result<(), EngineError> {
        admin::set_vault(env, vault)
    }

    pub fn set_pool(env: Env, pool: Address) -> Result<(), EngineError> {
        admin::set_pool(env, pool)
    }

    pub fn set_oracle(env: Env, oracle: Address) -> Result<(), EngineError> {
        admin::set_oracle(env, oracle)
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        storage::get_admin(&env)
    }

    pub fn get_vault(env: Env) -> Option<Address> {
        storage::get_vault(&env)
    }

    pub fn get_pool(env: Env) -> Option<Address> {
        storage::get_pool(&env)
    }

    pub fn get_oracle(env: Env) -> Option<Address> {
        storage::get_oracle(&env)
    }

    pub fn liquidate(
        env: Env,
        liquidator: Address,
        user: Address,
        max_repay_amount: i128,
    ) -> Result<LiquidationResult, EngineError> {
        liquidate::liquidate(env, liquidator, user, max_repay_amount)
    }

    pub fn is_liquidatable(env: Env, user: Address) -> bool {
        match liquidate::is_liquidatable(env.clone(), user) {
            Ok(flag) => flag,
            Err(e) => soroban_sdk::panic_with_error!(&env, e),
        }
    }

    pub fn calculate_bonus(env: Env, repaid_amount: i128) -> i128 {
        match liquidate::calculate_bonus(env.clone(), repaid_amount) {
            Ok(bonus) => bonus,
            Err(e) => soroban_sdk::panic_with_error!(&env, e),
        }
    }

    pub fn calculate_partial_repayment(env: Env, user: Address) -> i128 {
        match liquidate::calculate_partial_repayment(env.clone(), user) {
            Ok(amount) => amount,
            Err(e) => soroban_sdk::panic_with_error!(&env, e),
        }
    }
}

mod admin;
mod errors;
mod events;
mod liquidate;
mod storage;
#[cfg(test)]
mod tests;
mod types;
