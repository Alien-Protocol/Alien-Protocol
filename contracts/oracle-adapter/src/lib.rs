#![no_std]

// RedStone's processor APIs are expressed in terms of `alloc` collections (`alloc::vec::Vec`,
// `alloc::vec!`) and this crate is `no_std`, so the `alloc` crate must be linked explicitly.
// It is the only allocator entry point in the crate; Soroban collection types (`soroban_sdk::Vec`)
// are used everywhere else, which keeps the WASM footprint minimal.
extern crate alloc;

use soroban_sdk::{contract, contractimpl, Address, Bytes, Env, Symbol, Vec};

mod errors;
pub use errors::OracleError;

mod events;
pub mod oracle;
mod storage;
mod types;

pub use types::{DataKey, PriceData};

#[contract]
pub struct OracleContract;

/// Load the admin address or panic with `NotInitialized` if the contract has not been initialized.
/// This is the single source of truth for the admin-load block across all panicking entry points.
fn admin_or_panic(env: &Env) -> Address {
    match storage::get_admin(env) {
        Some(addr) => addr,
        None => soroban_sdk::panic_with_error!(env, OracleError::NotInitialized),
    }
}

#[contractimpl]
impl OracleContract {
    pub fn initialize(env: Env, admin: Address, staleness_threshold: u64) {
        if storage::is_initialized(&env) {
            soroban_sdk::panic_with_error!(&env, OracleError::AlreadyInitialized);
        }
        admin.require_auth();
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
        if let Some(feed_id) = storage::get_asset_feed(&env, &asset) {
            if let Some(feed_price) = oracle::storage::get_feed_price(&env, &feed_id) {
                return Some(feed_price);
            }
        }
        storage::get_price(&env, &asset)
    }

    pub fn is_price_fresh(env: Env, asset: Address) -> bool {
        let price_data = match Self::get_price(env.clone(), asset) {
            Some(data) => data,
            None => return false,
        };
        let threshold = match storage::get_staleness_threshold(&env) {
            Some(t) => t,
            None => return false,
        };
        let ledger_time = env.ledger().timestamp();
        let price_ts_seconds = if price_data.timestamp > 100_000_000_000 {
            price_data.timestamp / 1_000
        } else {
            price_data.timestamp
        };
        match ledger_time.checked_sub(price_ts_seconds) {
            Some(delta) => delta <= threshold,
            None => false,
        }
    }

    pub fn get_price_or_fail(env: Env, asset: Address) -> PriceData {
        let price = match Self::get_price(env.clone(), asset.clone()) {
            Some(price) => price,
            None => soroban_sdk::panic_with_error!(&env, OracleError::PriceNotFound),
        };
        if !Self::is_price_fresh(env.clone(), asset) {
            soroban_sdk::panic_with_error!(&env, OracleError::StalePrice);
        }
        price
    }

    pub fn set_price(env: Env, caller: Address, asset: Address, price: i128, timestamp: u64) {
        let admin = admin_or_panic(&env);
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

        if price <= 0 {
            soroban_sdk::panic_with_error!(&env, OracleError::InvalidPrice);
        }
        if timestamp == 0 {
            soroban_sdk::panic_with_error!(&env, OracleError::InvalidTimestamp);
        }

        // Reject timestamps that are not strictly newer than the last stored price.
        // This prevents feeders from overwriting a fresh price with stale data.
        if let Some(existing) = storage::get_price(&env, &asset) {
            if timestamp <= existing.timestamp {
                soroban_sdk::panic_with_error!(&env, OracleError::InvalidTimestamp);
            }
        }

        // Reject timestamps that lie in the future relative to the current ledger.
        // A price whose timestamp exceeds ledger time is fabricated data.
        if timestamp > env.ledger().timestamp() {
            soroban_sdk::panic_with_error!(&env, OracleError::InvalidTimestamp);
        }

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

    pub fn get_staleness_threshold(env: Env) -> u64 {
        match storage::get_staleness_threshold(&env) {
            Some(t) => t,
            None => soroban_sdk::panic_with_error!(&env, OracleError::NotInitialized),
        }
    }

    pub fn set_staleness_threshold(env: Env, threshold: u64) {
        let admin = admin_or_panic(&env);
        admin.require_auth();

        if threshold == 0 {
            soroban_sdk::panic_with_error!(&env, OracleError::InvalidThreshold);
        }

        storage::set_staleness_threshold(&env, threshold);

        events::StalenessThresholdUpdated { threshold }.publish(&env);
    }

    pub fn set_admin(env: Env, new_admin: Address) {
        let current_admin = admin_or_panic(&env);
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

    pub fn pause(env: Env) {
        let admin = admin_or_panic(&env);
        admin.require_auth();

        if storage::is_paused(&env) {
            soroban_sdk::panic_with_error!(&env, OracleError::AlreadyPaused);
        }

        storage::set_paused(&env, true);
        events::Paused { by: admin }.publish(&env);
    }

    pub fn unpause(env: Env) {
        let admin = admin_or_panic(&env);
        admin.require_auth();

        if !storage::is_paused(&env) {
            soroban_sdk::panic_with_error!(&env, OracleError::NotPaused);
        }

        storage::set_paused(&env, false);
        events::Unpaused { by: admin }.publish(&env);
    }

    pub fn add_authorized_feeder(env: Env, feeder: Address) {
        let admin = admin_or_panic(&env);
        admin.require_auth();

        if storage::is_authorized_feeder(&env, &feeder) {
            soroban_sdk::panic_with_error!(&env, OracleError::AlreadyAuthorized);
        }

        storage::set_authorized_feeder(&env, &feeder);

        events::FeederAdded { feeder }.publish(&env);
    }

    pub fn remove_authorized_feeder(env: Env, feeder: Address) {
        let admin = admin_or_panic(&env);
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

        if threshold == 0 || threshold > signers.len() || threshold > (u8::MAX as u32) {
            return Err(OracleError::InvalidThreshold);
        }

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

    pub fn set_asset_feed(
        env: Env,
        caller: Address,
        asset: Address,
        feed_id: Symbol,
    ) -> Result<(), OracleError> {
        let admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => return Err(OracleError::NotInitialized),
        };
        if caller != admin {
            return Err(OracleError::Unauthorized);
        }
        caller.require_auth();

        storage::set_asset_feed(&env, &asset, &feed_id);
        Ok(())
    }

    pub fn get_asset_feed(env: Env, asset: Address) -> Option<Symbol> {
        storage::get_asset_feed(&env, &asset)
    }
}

#[cfg(test)]
mod tests;
