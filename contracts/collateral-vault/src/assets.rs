/// assets.rs — supported-asset allow-list management.
///
/// Every public function returns `Result<_, VaultError>`.  No `panic!`,
/// `expect`, or `unwrap` appears in production paths.
use crate::{config, errors::VaultError, events, storage};
use soroban_sdk::{Address, Env};

/// Add `asset` to the supported-asset allow-list.
///
/// Errors:
/// - `NotInitialized`  — vault has not been initialized.
/// - `AlreadySupported`— `asset` is already in the list.
pub fn add_supported_asset(env: Env, asset: Address) -> Result<(), VaultError> {
    let admin = config::require_admin(&env)?;
    admin.require_auth();

    if storage::is_supported_asset(&env, &asset) {
        return Err(VaultError::AlreadySupported);
    }

    storage::add_supported_asset(&env, &asset);

    events::AssetAdded { asset }.publish(&env);

    Ok(())
}

/// Remove `asset` from the supported-asset allow-list.
///
/// Removing an asset does not touch existing user positions; users who already
/// hold a balance of that asset can still withdraw it.
///
/// Errors:
/// - `NotInitialized` — vault has not been initialized.
/// - `AssetNotFound`  — `asset` is not in the list.
pub fn remove_supported_asset(env: Env, asset: Address) -> Result<(), VaultError> {
    let admin = config::require_admin(&env)?;
    admin.require_auth();

    if !storage::is_supported_asset(&env, &asset) {
        return Err(VaultError::AssetNotFound);
    }

    storage::remove_supported_asset(&env, &asset);

    events::AssetRemoved { asset }.publish(&env);

    Ok(())
}

/// Return whether `asset` is currently in the supported-asset allow-list.
///
/// This is a read-only query; it never fails.
pub fn is_supported_asset(env: Env, asset: Address) -> bool {
    storage::is_supported_asset(&env, &asset)
}
