#![no_std]
use soroban_sdk::{contract, contractevent, contractimpl, Address, Env};

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct AdminChanged {
    pub old_admin: Address,
    pub new_admin: Address,
}

mod errors;
mod events;
mod storage;
mod types;

pub use errors::OracleError;
pub use types::{DataKey, PriceData};

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    /// One-time setup: store admin and staleness threshold, emit Initialized event.
    /// Panics with "AlreadyInitialized" if called more than once.
    pub fn initialize(env: Env, admin: Address, staleness_threshold: u64) {
        if storage::is_initialized(&env) {
            panic!("AlreadyInitialized");
        }
        storage::set_admin(&env, &admin);
        storage::set_staleness_threshold(&env, staleness_threshold);
        events::Initialized {
            admin,
            staleness_threshold,
        }
        .publish(&env);
    }

    pub fn get_price(env: Env, asset: Address) -> Option<PriceData> {
        storage::get_price(&env, &asset)
    }

    pub fn is_price_fresh(env: Env, asset: Address) -> bool {
        let data = match storage::get_price(&env, &asset) {
            Some(data) => data,
            None => soroban_sdk::panic_with_error!(&env, OracleError::PriceNotFound),
        };
        let threshold = match storage::get_staleness_threshold(&env) {
            Some(threshold) => threshold,
            None => soroban_sdk::panic_with_error!(&env, OracleError::NotInitialized),
        };
        let current_time = env.ledger().timestamp();

        current_time <= data.timestamp || current_time - data.timestamp <= threshold
    }

    pub fn get_price_or_fail(env: Env, asset: Address) -> PriceData {
        let data = match storage::get_price(&env, &asset) {
            Some(data) => data,
            None => soroban_sdk::panic_with_error!(&env, OracleError::PriceNotFound),
        };

        if !Self::is_price_fresh(env.clone(), asset) {
            soroban_sdk::panic_with_error!(&env, OracleError::StalePrice);
        }

        data
    }

    pub fn set_price(env: Env, asset: Address, price: i128, timestamp: u64) {
        let caller = storage::get_admin(&env).expect("NotInitialized");
        caller.require_auth();
        let data = PriceData { price, timestamp };
        storage::set_price(&env, &asset, &data);
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        storage::get_admin(&env)
    }

    pub fn get_staleness_threshold(env: Env) -> Option<u64> {
        storage::get_staleness_threshold(&env)
    }

    pub fn set_admin(env: Env, new_admin: Address) {
        let current_admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => soroban_sdk::panic_with_error!(&env, OracleError::NotInitialized),
        };
        current_admin.require_auth();

        if current_admin == new_admin {
            soroban_sdk::panic_with_error!(&env, OracleError::AlreadyAdmin);
        }

        storage::set_admin(&env, &new_admin);

        AdminChanged {
            old_admin: current_admin,
            new_admin,
        }
        .publish(&env);
    }
}

mod test;
