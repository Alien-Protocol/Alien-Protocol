use soroban_sdk::{BytesN, Env};

use crate::{
    errors::VaultError,
    events::{ContractUpgraded, StorageMigrated},
    storage,
};

pub const CURRENT_CONTRACT_VERSION: u32 = 3;
pub const CURRENT_STORAGE_SCHEMA_VERSION: u32 = 3;

pub fn get_contract_version() -> u32 {
    CURRENT_CONTRACT_VERSION
}

pub fn get_storage_schema_version(env: &Env) -> u32 {
    storage::get_storage_schema_version(env)
}

pub fn upgrade(env: Env, wasm_hash: BytesN<32>, expected_version: u32) -> Result<(), VaultError> {
    let admin = storage::get_admin(&env).ok_or(VaultError::NotInitialized)?;
    admin.require_auth();

    env.storage().persistent().set(
        &crate::types::DataKey::PendingUpgrade,
        &(
            admin,
            CURRENT_CONTRACT_VERSION,
            expected_version,
            wasm_hash.clone(),
        ),
    );
    env.deployer().update_current_contract_wasm(wasm_hash);
    Ok(())
}

pub fn finalize_upgrade(env: Env) -> Result<(), VaultError> {
    let state = env.storage().persistent();
    let key = crate::types::DataKey::PendingUpgrade;
    let (admin, old_version, expected, wasm_hash): (soroban_sdk::Address, u32, u32, BytesN<32>) =
        state.get(&key).ok_or(VaultError::InvalidInputs)?;
    if expected != CURRENT_CONTRACT_VERSION
        || env.current_contract_address().executable()
            != Some(soroban_sdk::Executable::Wasm(wasm_hash.clone()))
    {
        return Err(VaultError::InvalidInputs);
    }
    state.remove(&key);
    storage::set_contract_version(&env, CURRENT_CONTRACT_VERSION);
    ContractUpgraded {
        actor: admin,
        old_contract_version: old_version,
        new_contract_version: CURRENT_CONTRACT_VERSION,
        wasm_hash,
    }
    .publish(&env);

    Ok(())
}

pub fn migrate(env: Env, target_storage_schema_version: u32) -> Result<(), VaultError> {
    let admin = storage::get_admin(&env).ok_or(VaultError::NotInitialized)?;
    admin.require_auth();

    let current = storage::get_storage_schema_version(&env);

    if target_storage_schema_version == current {
        return Err(VaultError::MigrationAlreadyApplied);
    }
    if target_storage_schema_version < current {
        return Err(VaultError::MigrationOutOfOrder);
    }
    if target_storage_schema_version > CURRENT_STORAGE_SCHEMA_VERSION {
        return Err(VaultError::MigrationSkipped);
    }

    let mut version = current;
    while version < target_storage_schema_version {
        let next = version + 1;
        migrate_step(&env, version, next)?;
        version = next;
    }

    storage::set_storage_schema_version(&env, version);

    StorageMigrated {
        actor: admin,
        old_storage_schema_version: current,
        new_storage_schema_version: version,
    }
    .publish(&env);

    Ok(())
}

fn migrate_step(env: &Env, from: u32, to: u32) -> Result<(), VaultError> {
    match (from, to) {
        (2, 3) => {
            storage::require_migrated_index(env);
            Ok(())
        }
        (1, 2) => Ok(()), // no-op migration: preserves all existing state
        _ => Err(VaultError::MigrationSkipped),
    }
}
