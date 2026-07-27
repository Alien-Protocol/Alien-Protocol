use soroban_sdk::{Address, Env};

use crate::errors::VaultError;
use crate::storage;
use crate::types::{UserPositionView, VaultStatus};
use crate::{OracleClient, PRICE_PRECISION};

/// Contract version reported by the read API.
const CONTRACT_VERSION: u32 = 1;

/// Return a bounded, non-panicking snapshot of the vault configuration.
pub fn get_config(env: &Env) -> Result<VaultStatus, VaultError> {
    if !storage::has_admin(env) {
        return Err(VaultError::NotInitialized);
    }

    Ok(VaultStatus {
        version: CONTRACT_VERSION,
        initialized: true,
        paused: storage::is_paused(env),
        admin: storage::get_admin(env),
        supported_assets_count: storage::get_supported_assets(env).len(),
        lending_pool: storage::get_pool(env).or_else(|| storage::_get_lending_pool(env)),
        oracle: storage::get_oracle(env),
        liquidation_engine: storage::get_liquidation_engine(env),
    })
}

/// Return a bounded, non-panicking snapshot of a user's position.
///
/// `total_collateral_value` is `None` when:
/// - The oracle address is not configured.
/// - Any individual asset price lookup returns `None`.
/// - Any arithmetic operation would overflow.
pub fn get_user_view(env: &Env, user: &Address) -> Result<UserPositionView, VaultError> {
    if !storage::has_admin(env) {
        return Err(VaultError::NotInitialized);
    }

    let position = storage::get_position(env, user);

    let (collateral_assets, total_collateral_value, position_count) = match position {
        Some(pos) => {
            let count = pos.collateral.len();
            let total = compute_total_value(env, &pos.collateral);
            (pos.collateral, total, count)
        }
        None => {
            let empty = soroban_sdk::Vec::new(env);
            (empty, None, 0)
        }
    };

    Ok(UserPositionView {
        user: user.clone(),
        collateral_assets,
        total_collateral_value,
        position_count,
    })
}

/// Return the full list of supported assets.
///
/// The list is bounded by design (assets must be explicitly added via
/// `add_supported_asset`), so no pagination is required.
pub fn get_supported_assets_list(env: &Env) -> Result<soroban_sdk::Vec<Address>, VaultError> {
    if !storage::has_admin(env) {
        return Err(VaultError::NotInitialized);
    }

    Ok(storage::get_supported_assets(env))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Try to sum the USD value of all collateral assets.
/// Returns `None` instead of panicking if any price is missing or arithmetic
/// would overflow.
fn compute_total_value(
    env: &Env,
    collateral: &soroban_sdk::Vec<crate::types::CollateralAsset>,
) -> Option<i128> {
    let oracle_address = storage::get_oracle(env)?;
    let oracle_client = OracleClient::new(env, &oracle_address);

    let mut total: i128 = 0;
    for item in collateral.iter() {
        let price_data = oracle_client.get_price(&item.asset)?;
        let item_value = item.amount.checked_mul(price_data.price)?;
        total = total.checked_add(item_value / PRICE_PRECISION)?;
    }

    Some(total)
}
