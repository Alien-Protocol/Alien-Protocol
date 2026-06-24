#![no_std]
use soroban_sdk::{contract, contracterror, contractevent, contractimpl, Address, Env};

#[contracterror]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OracleError {
    NotInitialized = 1,
    AlreadyAdmin = 2,
    OraclePaused = 3,
    AlreadyPaused = 4,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct AdminChanged {
    pub old_admin: Address,
    pub new_admin: Address,
}

mod events;
mod storage;
mod types;

pub use types::{DataKey, PriceData};

#[contract]
pub struct OracleContract;

#[contractimpl]
impl OracleContract {
    pub fn initialize(env: Env, admin: Address, staleness_threshold: u64) {
        if storage::is_initialized(&env) {
            panic!("AlreadyInitialized");
        }
        storage::set_admin(&env, &admin);
        storage::set_staleness_threshold(&env, staleness_threshold);
        storage::set_paused(&env, false);
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
        let price_data = match storage::get_price(&env, &asset) {
            Some(data) => data,
            None => return false,
        };
        let threshold = match storage::get_staleness_threshold(&env) {
            Some(t) => t,
            None => return false,
        };
        let ledger_time = env.ledger().timestamp();
        match ledger_time.checked_sub(price_data.timestamp) {
            Some(delta) => delta <= threshold,
            None => false,
        }
    }

    pub fn set_price(env: Env, asset: Address, price: i128, timestamp: u64) {
        let caller = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => soroban_sdk::panic_with_error!(&env, OracleError::NotInitialized),
        };
        caller.require_auth();

        if storage::is_paused(&env) {
            soroban_sdk::panic_with_error!(&env, OracleError::OraclePaused);
        }

        assert!(price > 0, "price must be positive");
        assert!(timestamp > 0, "timestamp must be positive");

        let data = PriceData { price, timestamp };
        storage::set_price(&env, &asset, &data);

        events::PriceUpdated {
            asset,
            price,
            timestamp,
        }
        .publish(&env);
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

    pub fn pause(env: Env) {
        let admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => soroban_sdk::panic_with_error!(&env, OracleError::NotInitialized),
        };
        admin.require_auth();

        if storage::is_paused(&env) {
            soroban_sdk::panic_with_error!(&env, OracleError::AlreadyPaused);
        }

        storage::set_paused(&env, true);
        events::Paused { by: admin }.publish(&env);
    }

    pub fn unpause(env: Env) {
        let admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => soroban_sdk::panic_with_error!(&env, OracleError::NotInitialized),
        };
        admin.require_auth();

        if !storage::is_paused(&env) {
            panic!("oracle is not paused");
        }

        storage::set_paused(&env, false);
        events::Unpaused { paused: false }.publish(&env);
    }
}

#[cfg(test)]
mod tests;

