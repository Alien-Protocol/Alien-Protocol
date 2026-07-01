#![no_std]
use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, Address, Bytes, Env, Symbol, Vec,
};

#[contracterror]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum OracleError {
    NotInitialized = 1,
    AlreadyAdmin = 2,
    OraclePaused = 3,
    AlreadyPaused = 4,
    FeederNotFound = 5,
    NotPaused = 6,
    AlreadyAuthorized = 7,
    Unauthorized = 8,
    UnknownFeed = 9,
    InvalidPayload = 10,
    FeedNotWritten = 11,
    PriceNotFound = 12,
    StalePrice = 13,
    ThresholdZero = 14,
}

#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct AdminChanged {
    pub old_admin: Address,
    pub new_admin: Address,
}
#[contractevent]
#[derive(Clone, Debug, PartialEq)]
pub struct FeederAdded {
    pub feeder: Address,
}

mod events;
pub mod oracle;
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

    pub fn get_price_or_fail(env: Env, asset: Address) -> PriceData {
        let price_data = match storage::get_price(&env, &asset) {
            Some(data) => data,
            None => soroban_sdk::panic_with_error!(&env, OracleError::PriceNotFound),
        };
        let threshold = match storage::get_staleness_threshold(&env) {
            Some(t) => t,
            None => soroban_sdk::panic_with_error!(&env, OracleError::NotInitialized),
        };
        let ledger_time = env.ledger().timestamp();
        let is_fresh = match ledger_time.checked_sub(price_data.timestamp) {
            Some(delta) => delta <= threshold,
            None => false,
        };
        if !is_fresh {
            soroban_sdk::panic_with_error!(&env, OracleError::StalePrice);
        }
        price_data
    }

    pub fn set_price(env: Env, caller: Address, asset: Address, price: i128, timestamp: u64) {
        let admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => soroban_sdk::panic_with_error!(&env, OracleError::NotInitialized),
        };
        let is_admin = caller == admin;
        let is_authorized_feeder = storage::is_authorized_feeder(&env, &caller);

        if is_admin || is_authorized_feeder {
            caller.require_auth();
        } else {
            soroban_sdk::panic_with_error!(&env, OracleError::Unauthorized);
        }

        if storage::is_paused(&env) {
            soroban_sdk::panic_with_error!(&env, OracleError::OraclePaused);
        }

        assert!(price > 0, "price must be positive");
        assert!(timestamp > 0, "timestamp must be positive");

        let data = PriceData {
            price,
            timestamp,
            write_timestamp: env.ledger().timestamp(),
        };
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

        if threshold == 0 {
            soroban_sdk::panic_with_error!(&env, OracleError::ThresholdZero);
        }

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
            soroban_sdk::panic_with_error!(&env, OracleError::NotPaused);
        }

        storage::set_paused(&env, false);
        events::Unpaused { by: admin }.publish(&env);
    }

    pub fn add_authorized_feeder(env: Env, feeder: Address) {
        let admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => soroban_sdk::panic_with_error!(&env, OracleError::NotInitialized),
        };
        admin.require_auth();

        if storage::is_authorized_feeder(&env, &feeder) {
            soroban_sdk::panic_with_error!(&env, OracleError::AlreadyAuthorized);
        }

        storage::set_authorized_feeder(&env, &feeder);

        FeederAdded { feeder }.publish(&env);
    }

    pub fn remove_authorized_feeder(env: Env, feeder: Address) {
        let admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => soroban_sdk::panic_with_error!(&env, OracleError::NotInitialized),
        };
        admin.require_auth();

        if !storage::has_authorized_feeder(&env, &feeder) {
            soroban_sdk::panic_with_error!(&env, OracleError::FeederNotFound);
        }

        storage::remove_authorized_feeder(&env, &feeder);

        events::FeederRemoved { feeder }.publish(&env);
    }

    pub fn is_authorized_feeder(env: Env, feeder: Address) -> bool {
        storage::is_authorized_feeder(&env, &feeder)
    }

    pub fn get_prices(
        env: Env,
        feed_ids: Vec<Symbol>,
        payload: Bytes,
    ) -> Result<(u64, Vec<i128>), OracleError> {
        oracle::pull::get_prices(env, feed_ids, payload)
    }

    pub fn write_prices(
        env: Env,
        caller: Address,
        feed_ids: Vec<Symbol>,
        payload: Bytes,
    ) -> Result<(), OracleError> {
        oracle::push::write_prices(env, caller, feed_ids, payload)
    }

    pub fn read_prices(env: Env, feed_ids: Vec<Symbol>) -> Result<Vec<PriceData>, OracleError> {
        oracle::push::read_prices(env, feed_ids)
    }

    pub fn set_redstone_config(
        env: Env,
        caller: Address,
        signers: Vec<Bytes>,
        threshold: u32,
    ) -> Result<(), OracleError> {
        let admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => return Err(OracleError::NotInitialized),
        };
        if caller != admin {
            return Err(OracleError::Unauthorized);
        }
        caller.require_auth();

        oracle::storage::set_redstone_signers(&env, &signers);
        oracle::storage::set_redstone_threshold(&env, threshold);
        Ok(())
    }

    pub fn get_redstone_config(env: Env) -> Result<(Vec<Bytes>, u32), OracleError> {
        if !oracle::storage::is_redstone_initialized(&env) {
            return Err(OracleError::NotInitialized);
        }
        let signers = oracle::storage::get_redstone_signers(&env).unwrap_or(Vec::new(&env));
        let threshold = oracle::storage::get_redstone_threshold(&env).unwrap_or(0);
        Ok((signers, threshold))
    }
}

#[cfg(test)]
mod tests;
