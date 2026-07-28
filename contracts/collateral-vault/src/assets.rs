//! Asset lifecycle domain.
//!
//! Manages the set of collateral assets the vault accepts and the
//! address of the liquidation engine that may seize those assets.

use crate::admin::require_admin;
use crate::errors::VaultError;
use crate::events;
use crate::storage;
use soroban_sdk::{Address, Env};

pub fn add_supported_asset(env: &Env, asset: Address) {
    let admin = require_admin(env);
    admin.require_auth();

    if storage::is_supported_asset(env, &asset) {
        soroban_sdk::panic_with_error!(env, VaultError::AlreadySupported);
    }

    storage::add_supported_asset(env, &asset);
    events::AssetAdded { asset }.publish(env);
}

pub fn remove_supported_asset(env: &Env, asset: Address) {
    let admin = require_admin(env);
    admin.require_auth();

    if !storage::is_supported_asset(env, &asset) {
        soroban_sdk::panic_with_error!(env, VaultError::AssetNotFound);
    }

    storage::remove_supported_asset(env, &asset);
    events::AssetRemoved { asset }.publish(env);
}

pub fn set_liquidation_engine(env: &Env, engine: Address) {
    let admin = require_admin(env);
    admin.require_auth();
    storage::set_liquidation_engine(env, &engine);
    events::LiquidationEngineSet { engine }.publish(env);
}

pub fn is_supported_asset(env: &Env, asset: &Address) -> bool {
    storage::is_supported_asset(env, asset)
}
