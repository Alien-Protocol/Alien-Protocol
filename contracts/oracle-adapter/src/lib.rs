#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env};

mod events;
mod storage;
mod types;

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
}

mod test;
