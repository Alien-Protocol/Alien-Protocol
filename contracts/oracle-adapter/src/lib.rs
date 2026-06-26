#![no_std]
use soroban_sdk::{contract, contracterror, contractimpl, Address, Env};

#[contracterror]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OracleError {
    NotInitialized = 1,
    AlreadyAdmin = 2,
    AlreadyFeeder = 3,
    NotFeeder = 4,
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

    pub fn set_staleness_threshold(env: Env, threshold: u64) {
        let admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => soroban_sdk::panic_with_error!(&env, OracleError::NotInitialized),
        };
        admin.require_auth();

        assert!(threshold > 0, "threshold must be positive");

        storage::set_staleness_threshold(&env, threshold);

        events::StalenessThresholdUpdated { threshold }.publish(&env);
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

        events::AdminChanged {
            old_admin: current_admin,
            new_admin,
        }
        .publish(&env);
    }

    pub fn add_feeder(env: Env, feeder: Address) {
        let admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => soroban_sdk::panic_with_error!(&env, OracleError::NotInitialized),
        };
        admin.require_auth();

        if storage::has_feeder(&env, &feeder) {
            soroban_sdk::panic_with_error!(&env, OracleError::AlreadyFeeder);
        }

        storage::add_feeder(&env, &feeder);

        events::FeederAdded { feeder }.publish(&env);
    }

    pub fn remove_feeder(env: Env, feeder: Address) {
        let admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => soroban_sdk::panic_with_error!(&env, OracleError::NotInitialized),
        };
        admin.require_auth();

        if !storage::has_feeder(&env, &feeder) {
            soroban_sdk::panic_with_error!(&env, OracleError::NotFeeder);
        }

        storage::remove_feeder(&env, &feeder);

        events::FeederRemoved { feeder }.publish(&env);
    }
}

mod test;
