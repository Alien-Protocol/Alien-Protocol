// SPDX-License-Identifier: Apache-2.0
#![no_std]
use soroban_sdk::{
    contract, contractevent, contractimpl, Address, Bytes, Env, Symbol, Vec,
};

mod errors;
pub use errors::OracleError;

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
    // Initialize the contract
    pub fn initialize(env: Env, admin: Address, staleness_threshold: u64) -> Result<(), OracleError> {
        if storage::is_initialized(&env) {
            return Err(OracleError::AlreadyInitialized.into());
        }
        storage::set_admin(&env, &admin);
        storage::set_staleness_threshold(&env, staleness_threshold);
        storage::set_paused(&env, false);
        events::Initialized { admin, staleness_threshold }.publish(&env);
        Ok(())
    }

    // Retrieve a price if it exists
    pub fn get_price(env: Env, asset: Address) -> Option<PriceData> {
        storage::get_price(&env, &asset)
    }

    // Check if a price is fresh
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

    // Get a price or fail with a typed error
    pub fn get_price_or_fail(env: Env, asset: Address) -> Result<PriceData, OracleError> {
        let price_data = match storage::get_price(&env, &asset) {
            Some(data) => data,
            None => return Err(OracleError::PriceNotFound.into()),
        };
        let threshold = match storage::get_staleness_threshold(&env) {
            Some(t) => t,
            None => return Err(OracleError::NotInitialized.into()),
        };
        let ledger_time = env.ledger().timestamp();
        let is_fresh = match ledger_time.checked_sub(price_data.timestamp) {
            Some(delta) => delta <= threshold,
            None => false,
        };
        if !is_fresh {
            return Err(OracleError::StalePrice.into());
        }
        Ok(price_data)
    }

    // Set a price for an asset
    pub fn set_price(
        env: Env,
        caller: Address,
        asset: Address,
        price: i128,
        timestamp: u64,
    ) -> Result<(), OracleError> {
        let admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => return Err(OracleError::NotInitialized.into()),
        };
        let is_admin = caller == admin;
        let is_authorized_feeder = storage::is_authorized_feeder(&env, &caller);
        if is_admin || is_authorized_feeder {
            caller.require_auth();
        } else {
            return Err(OracleError::Unauthorized.into());
        }
        if storage::is_paused(&env) {
            return Err(OracleError::OraclePaused.into());
        }
        if price <= 0 {
            return Err(OracleError::InvalidPrice.into());
        }
        if timestamp == 0 {
            return Err(OracleError::InvalidTimestamp.into());
        }
        let data = PriceData {
            price,
            timestamp,
            write_timestamp: env.ledger().timestamp(),
        };
        storage::set_price(&env, &asset, &data);
        events::PriceUpdated { asset, price, timestamp }.publish(&env);
        Ok(())
    }

    // Get the admin address
    pub fn get_admin(env: Env) -> Option<Address> {
        storage::get_admin(&env)
    }

    // Get staleness threshold
    pub fn get_staleness_threshold(env: Env) -> Option<u64> {
        storage::get_staleness_threshold(&env)
    }

    // Update admin address
    pub fn set_admin(env: Env, new_admin: Address) -> Result<(), OracleError> {
        let current_admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => return Err(OracleError::NotInitialized.into()),
        };
        current_admin.require_auth();
        if current_admin == new_admin {
            return Err(OracleError::AlreadyAdmin.into());
        }
        storage::set_admin(&env, &new_admin);
        AdminChanged { old_admin: current_admin, new_admin }.publish(&env);
        Ok(())
    }

    // Pause the contract
    pub fn pause(env: Env) -> Result<(), OracleError> {
        let admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => return Err(OracleError::NotInitialized.into()),
        };
        admin.require_auth();
        if storage::is_paused(&env) {
            return Err(OracleError::AlreadyPaused.into());
        }
        storage::set_paused(&env, true);
        events::Paused { by: admin }.publish(&env);
        Ok(())
    }

    // Unpause the contract
    pub fn unpause(env: Env) -> Result<(), OracleError> {
        let admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => return Err(OracleError::NotInitialized.into()),
        };
        admin.require_auth();
        if !storage::is_paused(&env) {
            return Err(OracleError::NotPaused.into());
        }
        storage::set_paused(&env, false);
        events::Unpaused { by: admin }.publish(&env);
        Ok(())
    }

    // Add an authorized feeder
    pub fn add_authorized_feeder(env: Env, feeder: Address) -> Result<(), OracleError> {
        let admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => return Err(OracleError::NotInitialized.into()),
        };
        admin.require_auth();
        if storage::is_authorized_feeder(&env, &feeder) {
            return Err(OracleError::AlreadyAuthorized.into());
        }
        storage::set_authorized_feeder(&env, &feeder);
        FeederAdded { feeder }.publish(&env);
        Ok(())
    }

    // Remove an authorized feeder
    pub fn remove_authorized_feeder(env: Env, feeder: Address) -> Result<(), OracleError> {
        let admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => return Err(OracleError::NotInitialized.into()),
        };
        admin.require_auth();
        if !storage::has_authorized_feeder(&env, &feeder) {
            return Err(OracleError::FeederNotFound.into());
        }
        storage::remove_authorized_feeder(&env, &feeder);
        events::FeederRemoved { feeder }.publish(&env);
        Ok(())
    }

    // Check if feeder is authorized
    pub fn is_authorized_feeder(env: Env, feeder: Address) -> bool {
        storage::is_authorized_feeder(&env, &feeder)
    }

    // External oracle interactions
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

    pub fn read_prices(
        env: Env,
        feed_ids: Vec<Symbol>,
    ) -> Result<Vec<PriceData>, OracleError> {
        oracle::push::read_prices(env, feed_ids)
    }

    // Redstone config management
    pub fn set_redstone_config(
        env: Env,
        caller: Address,
        signers: Vec<Bytes>,
        threshold: u32,
    ) -> Result<(), OracleError> {
        let admin = match storage::get_admin(&env) {
            Some(addr) => addr,
            None => return Err(OracleError::NotInitialized.into()),
        };
        if caller != admin {
            return Err(OracleError::Unauthorized.into());
        }
        caller.require_auth();
        oracle::storage::set_redstone_signers(&env, &signers);
        oracle::storage::set_redstone_threshold(&env, threshold);
        Ok(())
    }

    pub fn get_redstone_config(env: Env) -> Result<(Vec<Bytes>, u32), OracleError> {
        if !oracle::storage::is_redstone_initialized(&env) {
            return Err(OracleError::NotInitialized.into());
        }
        let signers = oracle::storage::get_redstone_signers(&env).unwrap_or(Vec::new(&env));
        let threshold = oracle::storage::get_redstone_threshold(&env).unwrap_or(0);
        Ok((signers, threshold))
    }
}

#[cfg(test)]
mod tests;
