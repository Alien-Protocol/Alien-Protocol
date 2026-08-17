use crate::{errors::VaultError, events, storage, types::AssetConfig};
use soroban_sdk::{Address, Env};

pub fn add_supported_asset(env: Env, asset: Address) {
    let admin = storage::get_admin(&env).expect("not initialized");
    admin.require_auth();

    if storage::is_supported_asset(&env, &asset) {
        soroban_sdk::panic_with_error!(&env, VaultError::AlreadySupported);
    }

    storage::add_supported_asset(&env, &asset);
    storage::set_asset_config(&env, &asset, &AssetConfig::default());

    events::AssetAdded { asset }.publish(&env);
}

pub fn set_asset_config(
    env: Env,
    asset: Address,
    token_decimals: u32,
    oracle_price_decimals: u32,
    max_ltv_bps: u32,
    liquidation_threshold_bps: u32,
) -> Result<(), VaultError> {
    let admin = storage::get_admin(&env).expect("not initialized");
    admin.require_auth();

    if !storage::is_supported_asset(&env, &asset) {
        return Err(VaultError::UnsupportedAsset);
    }

    if storage::has_open_position(&env, &asset) {
        let existing = storage::get_asset_config(&env, &asset).unwrap_or(AssetConfig::default());
        if existing.token_decimals != token_decimals
            || existing.oracle_price_decimals != oracle_price_decimals
            || existing.max_ltv_bps != max_ltv_bps
            || existing.liquidation_threshold_bps != liquidation_threshold_bps
        {
            return Err(VaultError::ImmutableMetadata);
        }
    }

    crate::risk::validate_asset_risk_config(
        token_decimals,
        oracle_price_decimals,
        max_ltv_bps,
        liquidation_threshold_bps,
    )?;
    storage::set_asset_config(
        &env,
        &asset,
        &AssetConfig {
            token_decimals,
            oracle_price_decimals,
            max_ltv_bps,
            liquidation_threshold_bps,
        },
    );

    Ok(())
}

pub fn get_asset_config(env: Env, asset: Address) -> Result<AssetConfig, VaultError> {
    if !storage::is_supported_asset(&env, &asset) {
        return Err(VaultError::UnsupportedAsset);
    }
    Ok(storage::get_asset_config_or_default(&env, &asset))
}

pub fn remove_supported_asset(env: Env, asset: Address) -> Result<(), VaultError> {
    let admin = storage::get_admin(&env).expect("not initialized");
    admin.require_auth();

    if !storage::is_supported_asset(&env, &asset) {
        return Err(VaultError::AssetNotFound);
    }

    storage::remove_supported_asset(&env, &asset);

    events::AssetRemoved { asset }.publish(&env);
    Ok(())
}

pub fn is_supported_asset(env: Env, asset: Address) -> bool {
    storage::is_supported_asset(&env, &asset)
}
