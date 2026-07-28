use crate::{errors::VaultError, events, storage, types::AssetStatus};
use soroban_sdk::{Address, Env};

pub fn add_supported_asset(env: Env, asset: Address) {
    let admin = storage::get_admin(&env).expect("not initialized");
    admin.require_auth();

    // Reject duplicates regardless of current status: re-activating a delisted
    // asset must go through `delist_supported_asset` reversal, not re-add.
    if storage::is_known_asset(&env, &asset) {
        soroban_sdk::panic_with_error!(&env, VaultError::AlreadySupported);
    }

    storage::add_supported_asset(&env, &asset);

    events::AssetAdded { asset }.publish(&env);
}

/// Transition the asset to `DepositDisabled`.
///
/// - Blocks new deposits immediately.
/// - Existing positions remain withdrawable, priceable, and liquidatable.
/// - The price/risk configuration is NOT removed.
pub fn delist_supported_asset(env: Env, asset: Address) {
    let admin = storage::get_admin(&env).expect("not initialized");
    admin.require_auth();

    match storage::get_asset_status(&env, &asset) {
        None => soroban_sdk::panic_with_error!(&env, VaultError::AssetNotFound),
        Some(AssetStatus::DepositDisabled) => {
            // Already delisted — treat as a no-op to stay idempotent.
        }
        Some(AssetStatus::Active) => {
            storage::delist_supported_asset(&env, &asset);
            events::AssetDelisted { asset }.publish(&env);
        }
    }
}

/// Permanently remove an asset from the registry.
///
/// This is a hard-delete: the asset's status entry and its entry in the
/// `SupportedAssets` list are both erased.  It is only permitted when **no
/// user balance remains** for the asset across all tracked users, because
/// removing configuration while positions exist would make those positions
/// un-priceable and un-liquidatable.
pub fn remove_supported_asset(env: Env, asset: Address) {
    let admin = storage::get_admin(&env).expect("not initialized");
    admin.require_auth();

    if !storage::is_known_asset(&env, &asset) {
        soroban_sdk::panic_with_error!(&env, VaultError::AssetNotFound);
    }

    // Guard: refuse removal while any user still holds a balance.
    let all_users = storage::get_position_index(&env);
    for user in all_users.iter() {
        let bal = storage::get_position_balance(&env, &user, &asset);
        if bal > 0 {
            soroban_sdk::panic_with_error!(&env, VaultError::AssetHasOpenPositions);
        }
    }

    storage::remove_supported_asset(&env, &asset);

    events::AssetRemoved { asset }.publish(&env);
}

pub fn is_supported_asset(env: Env, asset: Address) -> bool {
    storage::is_supported_asset(&env, &asset)
}

/// Returns the raw lifecycle status of an asset.
pub fn get_asset_status(env: Env, asset: Address) -> Option<AssetStatus> {
    storage::get_asset_status(&env, &asset)
}
