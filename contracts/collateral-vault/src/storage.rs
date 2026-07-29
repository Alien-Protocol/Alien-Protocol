use crate::types::{CollateralAsset, DataKey, Position};
use soroban_sdk::{Address, Env, Vec};

// ─────────────────────────────────────────────────────────────────────────────
// Admin / pause / addresses
// ─────────────────────────────────────────────────────────────────────────────

pub fn has_admin(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get::<_, Address>(&DataKey::Admin)
        .is_some()
}

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::Admin)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().persistent().set(&DataKey::Admin, admin);
}

pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::Paused)
        .unwrap_or(false)
}

pub fn set_paused(env: &Env, paused: bool) {
    env.storage().persistent().set(&DataKey::Paused, &paused);
}

pub fn get_lending_pool(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::LendingPool)
}

pub fn set_lending_pool(env: &Env, lending_pool: &Address) {
    env.storage()
        .persistent()
        .set(&DataKey::LendingPool, lending_pool);
}

pub fn get_oracle(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::Oracle)
}

pub fn set_oracle(env: &Env, oracle: &Address) {
    env.storage().persistent().set(&DataKey::Oracle, oracle);
}

pub fn get_liquidation_engine(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::LiquidationEngine)
}

pub fn set_liquidation_engine(env: &Env, engine: &Address) {
    env.storage()
        .persistent()
        .set(&DataKey::LiquidationEngine, engine);
}

pub fn get_pool(env: &Env) -> Option<Address> {
    env.storage().persistent().get(&DataKey::Pool)
}

pub fn set_pool(env: &Env, pool: &Address) {
    env.storage().persistent().set(&DataKey::Pool, pool);
}

// ─────────────────────────────────────────────────────────────────────────────
// Supported-asset index (slot-based, O(1) add/remove via swap-and-pop)
// ─────────────────────────────────────────────────────────────────────────────

pub fn is_supported_asset(env: &Env, asset: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::SupportedAsset(asset.clone()))
        .unwrap_or(false)
}

pub fn supported_asset_count(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::SupportedAssetCount)
        .unwrap_or(0u32)
}

pub fn get_supported_asset_at(env: &Env, slot: u32) -> Option<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::SupportedAssetAt(slot))
}

pub fn add_supported_asset(env: &Env, asset: &Address) {
    if env
        .storage()
        .persistent()
        .get::<_, u32>(&DataKey::SupportedAssetSlot(asset.clone()))
        .is_some()
    {
        return;
    }

    env.storage()
        .persistent()
        .set(&DataKey::SupportedAsset(asset.clone()), &true);

    let count: u32 = supported_asset_count(env);
    env.storage()
        .persistent()
        .set(&DataKey::SupportedAssetAt(count), asset);
    env.storage()
        .persistent()
        .set(&DataKey::SupportedAssetSlot(asset.clone()), &count);
    env.storage()
        .persistent()
        .set(&DataKey::SupportedAssetCount, &(count + 1));
}

pub fn remove_supported_asset(env: &Env, asset: &Address) {
    let count: u32 = supported_asset_count(env);
    if count == 0 {
        return;
    }

    let slot: u32 = match env
        .storage()
        .persistent()
        .get(&DataKey::SupportedAssetSlot(asset.clone()))
    {
        Some(s) => s,
        None => return,
    };

    env.storage()
        .persistent()
        .remove(&DataKey::SupportedAsset(asset.clone()));

    let last_slot = count - 1;

    if slot != last_slot {
        let last_asset: Address = env
            .storage()
            .persistent()
            .get(&DataKey::SupportedAssetAt(last_slot))
            .unwrap();

        env.storage()
            .persistent()
            .set(&DataKey::SupportedAssetAt(slot), &last_asset);
        env.storage()
            .persistent()
            .set(&DataKey::SupportedAssetSlot(last_asset), &slot);
    }

    env.storage()
        .persistent()
        .remove(&DataKey::SupportedAssetAt(last_slot));
    env.storage()
        .persistent()
        .remove(&DataKey::SupportedAssetSlot(asset.clone()));
    env.storage()
        .persistent()
        .set(&DataKey::SupportedAssetCount, &last_slot);
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-(user,asset) balance
// ─────────────────────────────────────────────────────────────────────────────

pub fn get_position_balance(env: &Env, user: &Address, asset: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::Position(user.clone(), asset.clone()))
        .unwrap_or(0)
}

pub fn set_position_balance(env: &Env, user: &Address, asset: &Address, balance: i128) {
    if balance == 0 {
        env.storage()
            .persistent()
            .remove(&DataKey::Position(user.clone(), asset.clone()));
    } else {
        env.storage()
            .persistent()
            .set(&DataKey::Position(user.clone(), asset.clone()), &balance);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-user asset index (slot-based, O(1) add/remove)
// ─────────────────────────────────────────────────────────────────────────────

pub fn user_asset_count(env: &Env, user: &Address) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::UserAssetCount(user.clone()))
        .unwrap_or(0u32)
}

pub fn get_user_asset_at(env: &Env, user: &Address, slot: u32) -> Option<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::UserAssetAt(user.clone(), slot))
}

pub fn add_user_asset(env: &Env, user: &Address, asset: &Address) {
    if env
        .storage()
        .persistent()
        .get::<_, u32>(&DataKey::UserAssetSlot(user.clone(), asset.clone()))
        .is_some()
    {
        return;
    }

    let count: u32 = user_asset_count(env, user);
    env.storage()
        .persistent()
        .set(&DataKey::UserAssetAt(user.clone(), count), asset);
    env.storage()
        .persistent()
        .set(&DataKey::UserAssetSlot(user.clone(), asset.clone()), &count);
    env.storage()
        .persistent()
        .set(&DataKey::UserAssetCount(user.clone()), &(count + 1));
}

pub fn remove_user_asset(env: &Env, user: &Address, asset: &Address) {
    let count: u32 = user_asset_count(env, user);
    if count == 0 {
        return;
    }

    let slot: u32 = match env
        .storage()
        .persistent()
        .get(&DataKey::UserAssetSlot(user.clone(), asset.clone()))
    {
        Some(s) => s,
        None => return,
    };

    let last_slot = count - 1;

    if slot != last_slot {
        let last_asset: Address = env
            .storage()
            .persistent()
            .get(&DataKey::UserAssetAt(user.clone(), last_slot))
            .unwrap();

        env.storage()
            .persistent()
            .set(&DataKey::UserAssetAt(user.clone(), slot), &last_asset);
        env.storage()
            .persistent()
            .set(&DataKey::UserAssetSlot(user.clone(), last_asset), &slot);
    }

    env.storage()
        .persistent()
        .remove(&DataKey::UserAssetAt(user.clone(), last_slot));
    env.storage()
        .persistent()
        .remove(&DataKey::UserAssetSlot(user.clone(), asset.clone()));
    env.storage()
        .persistent()
        .set(&DataKey::UserAssetCount(user.clone()), &last_slot);
}

// ─────────────────────────────────────────────────────────────────────────────
// Global position (user) index (slot-based, O(1) add/remove)
// ─────────────────────────────────────────────────────────────────────────────

pub fn position_count(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::PositionCount)
        .unwrap_or(0u32)
}

pub fn get_position_at(env: &Env, slot: u32) -> Option<Address> {
    env.storage().persistent().get(&DataKey::PositionAt(slot))
}

pub fn user_in_position_index(env: &Env, user: &Address) -> bool {
    env.storage()
        .persistent()
        .get::<_, u32>(&DataKey::PositionSlot(user.clone()))
        .is_some()
}

pub fn add_to_position_index(env: &Env, user: &Address) {
    if user_in_position_index(env, user) {
        return;
    }

    let count: u32 = position_count(env);
    env.storage()
        .persistent()
        .set(&DataKey::PositionAt(count), user);
    env.storage()
        .persistent()
        .set(&DataKey::PositionSlot(user.clone()), &count);
    env.storage()
        .persistent()
        .set(&DataKey::PositionCount, &(count + 1));
}

pub fn remove_from_position_index(env: &Env, user: &Address) {
    let count: u32 = position_count(env);
    if count == 0 {
        return;
    }

    let slot: u32 = match env
        .storage()
        .persistent()
        .get(&DataKey::PositionSlot(user.clone()))
    {
        Some(s) => s,
        None => return,
    };

    let last_slot = count - 1;

    if slot != last_slot {
        let last_user: Address = env
            .storage()
            .persistent()
            .get(&DataKey::PositionAt(last_slot))
            .unwrap();

        env.storage()
            .persistent()
            .set(&DataKey::PositionAt(slot), &last_user);
        env.storage()
            .persistent()
            .set(&DataKey::PositionSlot(last_user), &slot);
    }

    env.storage()
        .persistent()
        .remove(&DataKey::PositionAt(last_slot));
    env.storage()
        .persistent()
        .remove(&DataKey::PositionSlot(user.clone()));
    env.storage()
        .persistent()
        .set(&DataKey::PositionCount, &last_slot);
}

// ─────────────────────────────────────────────────────────────────────────────
// Position helpers
// ─────────────────────────────────────────────────────────────────────────────

pub fn get_position(env: &Env, user: &Address) -> Option<Position> {
    if !user_in_position_index(env, user) {
        return None;
    }

    let count = user_asset_count(env, user);
    let mut collateral: Vec<CollateralAsset> = Vec::new(env);

    for slot in 0..count {
        if let Some(asset) = get_user_asset_at(env, user, slot) {
            let balance = get_position_balance(env, user, &asset);
            if balance > 0 {
                collateral.push_back(CollateralAsset {
                    asset,
                    amount: balance,
                });
            }
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

pub fn get_position_index(env: &Env) -> soroban_sdk::Vec<Address> {
    let count = position_count(env);
    let mut result = soroban_sdk::Vec::new(env);
    for slot in 0..count {
        if let Some(user) = get_position_at(env, slot) {
            result.push_back(user);
        }
    }
    result
}

pub fn get_all_positions(env: &Env) -> soroban_sdk::Vec<Position> {
    let count = position_count(env);
    let mut positions = soroban_sdk::Vec::new(env);
    for slot in 0..count {
        if let Some(user) = get_position_at(env, slot) {
            if let Some(pos) = get_position(env, &user) {
                positions.push_back(pos);
            }
        }
    }
    positions
}

// ─────────────────────────────────────────────────────────────────────────────
// Contract / storage version (from upgrade module)
// ─────────────────────────────────────────────────────────────────────────────

pub const DEFAULT_CONTRACT_VERSION: u32 = 1;
pub const DEFAULT_STORAGE_SCHEMA_VERSION: u32 = 1;

pub fn get_contract_version(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::ContractVersion)
        .unwrap_or(DEFAULT_CONTRACT_VERSION)
}

pub fn set_contract_version(env: &Env, version: u32) {
    env.storage()
        .persistent()
        .set(&DataKey::ContractVersion, &version);
}

pub fn get_storage_schema_version(env: &Env) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::StorageSchemaVersion)
        .unwrap_or(DEFAULT_STORAGE_SCHEMA_VERSION)
}

pub fn set_storage_schema_version(env: &Env, version: u32) {
    env.storage()
        .persistent()
        .set(&DataKey::StorageSchemaVersion, &version);
}
