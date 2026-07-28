//! Storage access layer for the collateral-vault contract.
//!
//! See [`crate::constants`] for the full storage-class classification and TTL
//! policy documentation.
//!
//! ## Summary
//!
//! - **Instance storage**: `Admin`, `Paused`, `LendingPool`, `Oracle`,
//!   `LiquidationEngine`, `Pool`, `SupportedAssets`.  A single
//!   `extend_ttl` call on the instance covers all of these simultaneously.
//!
//! - **Persistent storage**: `SupportedAsset(Address)`,
//!   `Position(Address, Address)`, `PositionIndex`, `UserAssets(Address)`.
//!   Each key is extended individually on every read and write.

use crate::constants::{
    TTL_TARGET_INSTANCE, TTL_TARGET_PERSISTENT, TTL_THRESHOLD_INSTANCE, TTL_THRESHOLD_PERSISTENT,
};
use crate::types::{CollateralAsset, DataKey, Position};
use soroban_sdk::{Address, Env, Vec};

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Extend the contract instance TTL if it has fallen below the threshold.
#[inline]
fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(TTL_THRESHOLD_INSTANCE, TTL_TARGET_INSTANCE);
}

/// Extend a persistent key's TTL if it has fallen below the threshold.
#[inline]
fn bump_persistent(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, TTL_THRESHOLD_PERSISTENT, TTL_TARGET_PERSISTENT);
}

// ---------------------------------------------------------------------------
// Instance storage: Admin
// ---------------------------------------------------------------------------

/// Returns `true` if the contract has been initialized (admin key present).
pub fn has_admin(env: &Env) -> bool {
    let exists = env
        .storage()
        .instance()
        .get::<_, Address>(&DataKey::Admin)
        .is_some();
    if exists {
        bump_instance(env);
    }
    exists
}

pub fn get_admin(env: &Env) -> Option<Address> {
    let val = env.storage().instance().get(&DataKey::Admin);
    if val.is_some() {
        bump_instance(env);
    }
    val
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
    bump_instance(env);
}

// ---------------------------------------------------------------------------
// Instance storage: Paused
// ---------------------------------------------------------------------------

pub fn is_paused(env: &Env) -> bool {
    let val = env
        .storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false);
    bump_instance(env);
    val
}

pub fn set_paused(env: &Env, paused: bool) {
    env.storage().instance().set(&DataKey::Paused, &paused);
    bump_instance(env);
}

// ---------------------------------------------------------------------------
// Instance storage: LendingPool
// ---------------------------------------------------------------------------

pub fn _get_lending_pool(env: &Env) -> Option<Address> {
    let val = env.storage().instance().get(&DataKey::LendingPool);
    if val.is_some() {
        bump_instance(env);
    }
    val
}

pub fn set_lending_pool(env: &Env, lending_pool: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::LendingPool, lending_pool);
    bump_instance(env);
}

// ---------------------------------------------------------------------------
// Instance storage: Oracle
// ---------------------------------------------------------------------------

pub fn get_oracle(env: &Env) -> Option<Address> {
    let val = env.storage().instance().get(&DataKey::Oracle);
    if val.is_some() {
        bump_instance(env);
    }
    val
}

pub fn set_oracle(env: &Env, oracle: &Address) {
    env.storage().instance().set(&DataKey::Oracle, oracle);
    bump_instance(env);
}

// ---------------------------------------------------------------------------
// Instance storage: LiquidationEngine
// ---------------------------------------------------------------------------

pub fn get_liquidation_engine(env: &Env) -> Option<Address> {
    let val = env.storage().instance().get(&DataKey::LiquidationEngine);
    if val.is_some() {
        bump_instance(env);
    }
    val
}

pub fn set_liquidation_engine(env: &Env, engine: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::LiquidationEngine, engine);
    bump_instance(env);
}

// ---------------------------------------------------------------------------
// Instance storage: Pool
// ---------------------------------------------------------------------------

pub fn get_pool(env: &Env) -> Option<Address> {
    let val = env.storage().instance().get(&DataKey::Pool);
    if val.is_some() {
        bump_instance(env);
    }
    val
}

pub fn set_pool(env: &Env, pool: &Address) {
    env.storage().instance().set(&DataKey::Pool, pool);
    bump_instance(env);
}

// ---------------------------------------------------------------------------
// Instance storage: SupportedAssets list
// ---------------------------------------------------------------------------

pub fn get_supported_assets(env: &Env) -> Vec<Address> {
    let val = env
        .storage()
        .instance()
        .get(&DataKey::SupportedAssets)
        .unwrap_or_else(|| Vec::new(env));
    bump_instance(env);
    val
}

// ---------------------------------------------------------------------------
// Persistent storage: SupportedAsset(Address) per-asset flag
// ---------------------------------------------------------------------------

pub fn is_supported_asset(env: &Env, asset: &Address) -> bool {
    let key = DataKey::SupportedAsset(asset.clone());
    let val = env.storage().persistent().get(&key).unwrap_or(false);
    if val {
        bump_persistent(env, &key);
    }
    val
}

pub fn add_supported_asset(env: &Env, asset: &Address) {
    // Write per-asset flag to persistent storage
    let asset_key = DataKey::SupportedAsset(asset.clone());
    env.storage().persistent().set(&asset_key, &true);
    bump_persistent(env, &asset_key);

    // Update the assets list in instance storage
    let mut assets = get_supported_assets(env);
    if !assets.contains(asset) {
        assets.push_back(asset.clone());
        env.storage()
            .instance()
            .set(&DataKey::SupportedAssets, &assets);
        bump_instance(env);
    }
}

pub fn remove_supported_asset(env: &Env, asset: &Address) {
    // Remove per-asset persistent flag
    env.storage()
        .persistent()
        .remove(&DataKey::SupportedAsset(asset.clone()));

    // Update the assets list in instance storage
    let mut assets = get_supported_assets(env);
    let mut found_idx = None;
    for i in 0..assets.len() {
        if assets.get(i).unwrap() == *asset {
            found_idx = Some(i);
            break;
        }
    }
    if let Some(idx) = found_idx {
        assets.remove(idx);
        env.storage()
            .instance()
            .set(&DataKey::SupportedAssets, &assets);
        bump_instance(env);
    }
}

// ---------------------------------------------------------------------------
// Persistent storage: Position(user, asset) balance
// ---------------------------------------------------------------------------

pub fn get_position_balance(env: &Env, user: &Address, asset: &Address) -> i128 {
    let key = DataKey::Position(user.clone(), asset.clone());
    let val = env.storage().persistent().get(&key).unwrap_or(0);
    if val > 0 {
        bump_persistent(env, &key);
    }
    val
}

pub fn set_position_balance(env: &Env, user: &Address, asset: &Address, balance: i128) {
    let key = DataKey::Position(user.clone(), asset.clone());
    env.storage().persistent().set(&key, &balance);
    if balance > 0 {
        bump_persistent(env, &key);
    }
}

// ---------------------------------------------------------------------------
// Persistent storage: PositionIndex
// ---------------------------------------------------------------------------

pub fn get_position_index(env: &Env) -> Vec<Address> {
    let key = DataKey::PositionIndex;
    let val = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Vec::new(env));
    if !val.is_empty() {
        bump_persistent(env, &key);
    }
    val
}

pub fn add_to_position_index(env: &Env, user: &Address) {
    let key = DataKey::PositionIndex;
    let mut index = get_position_index(env);
    if !index.contains(user) {
        index.push_back(user.clone());
        env.storage().persistent().set(&key, &index);
        bump_persistent(env, &key);
    }
}

/// Remove a user from the position index (called when their balance reaches zero).
pub fn remove_from_position_index(env: &Env, user: &Address) {
    let key = DataKey::PositionIndex;
    let index = get_position_index(env);
    let mut new_index: Vec<Address> = Vec::new(env);
    for addr in index.iter() {
        if &addr != user {
            new_index.push_back(addr);
        }
    }
    env.storage().persistent().set(&key, &new_index);
    if !new_index.is_empty() {
        bump_persistent(env, &key);
    }
}

// ---------------------------------------------------------------------------
// Persistent storage: UserAssets(user)
// ---------------------------------------------------------------------------

/// Returns the list of assets a user has ever deposited into.
pub fn get_user_assets(env: &Env, user: &Address) -> Vec<Address> {
    let key = DataKey::UserAssets(user.clone());
    let val = env
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Vec::new(env));
    if !val.is_empty() {
        bump_persistent(env, &key);
    }
    val
}

pub fn add_user_asset(env: &Env, user: &Address, asset: &Address) {
    let key = DataKey::UserAssets(user.clone());
    let mut assets = get_user_assets(env, user);
    if !assets.contains(asset) {
        assets.push_back(asset.clone());
        env.storage().persistent().set(&key, &assets);
        bump_persistent(env, &key);
    }
}

/// Remove an asset from a user's tracked asset list (called when the balance hits zero).
pub fn remove_user_asset(env: &Env, user: &Address, asset: &Address) {
    let key = DataKey::UserAssets(user.clone());
    let assets = get_user_assets(env, user);
    let mut new_assets: Vec<Address> = Vec::new(env);
    for a in assets.iter() {
        if &a != asset {
            new_assets.push_back(a);
        }
    }
    env.storage().persistent().set(&key, &new_assets);
    if !new_assets.is_empty() {
        bump_persistent(env, &key);
    }
}

// ---------------------------------------------------------------------------
// Composite helpers
// ---------------------------------------------------------------------------

/// Build a `Position` for a user by loading all their non-zero balances.
pub fn get_position(env: &Env, user: &Address) -> Option<Position> {
    let index = get_position_index(env);
    if !index.contains(user) {
        return None;
    }

    let all_assets = get_user_assets(env, user);
    let mut collateral: Vec<CollateralAsset> = Vec::new(env);

    for asset in all_assets.iter() {
        let balance = get_position_balance(env, user, &asset);
        if balance > 0 {
            collateral.push_back(CollateralAsset {
                asset: asset.clone(),
                amount: balance,
            });
        }
    }

    if collateral.is_empty() {
        return None;
    }

    Some(Position {
        user: user.clone(),
        collateral,
    })
}

/// Returns all active positions (users with at least one non-zero balance).
pub fn get_all_positions(env: &Env) -> Vec<Position> {
    let index = get_position_index(env);
    let mut positions: Vec<Position> = Vec::new(env);
    for user in index.iter() {
        if let Some(position) = get_position(env, &user) {
            positions.push_back(position);
        }
    }
    positions
}
