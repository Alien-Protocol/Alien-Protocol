//! Asset lifecycle domain.

use crate::{errors::VaultError, events, storage};
use soroban_sdk::{Address, Env};

pub fn add_supported_asset(env: Env, asset: Address) {
    let admin = storage::get_admin(&env)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
    admin.require_auth();

    if storage::is_supported_asset(&env, &asset) {
        soroban_sdk::panic_with_error!(&env, VaultError::AlreadySupported);
    }

    storage::add_supported_asset(&env, &asset);
    events::AssetAdded { asset }.publish(&env);
}

pub fn remove_supported_asset(env: Env, asset: Address) {
    let admin = storage::get_admin(&env)
        .unwrap_or_else(|| soroban_sdk::panic_with_error!(&env, VaultError::NotInitialized));
    admin.require_auth();

    if !storage::is_supported_asset(&env, &asset) {
        soroban_sdk::panic_with_error!(&env, VaultError::AssetNotFound);
    }

    storage::remove_supported_asset(&env, &asset);
    events::AssetRemoved { asset }.publish(&env);
}

pub fn is_supported_asset(env: Env, asset: Address) -> bool {
    storage::is_supported_asset(&env, &asset)
}
