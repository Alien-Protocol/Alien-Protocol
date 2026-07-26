use soroban_sdk::{BytesN, Env};

use crate::{
    errors::VaultError,
    events::{ContractUpgraded, StorageMigrated},
    storage,
};

pub const CURRENT_CONTRACT_VERSION: u32 = 2;
pub const CURRENT_STORAGE_SCHEMA_VERSION: u32 = 2;

pub fn get_contract_version(env: &Env) -> u32 {
    storage::get_contract_version(env)
}

pub fn get_storage_schema_version(env: &Env) -> u32 {
    storage::get_storage_schema_version(env)
}

pub fn upgrade(env: Env, wasm_hash: BytesN<32>) -> Result<(), VaultError> {
    let admin = storage::get_admin(&env).ok_or(VaultError::NotInitialized)?;
    admin.require_auth();

    let old_version = storage::get_contract_version(&env);

    env.deployer()
        .update_current_contract_wasm(wasm_hash.clone());
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

fn migrate_step(_env: &Env, from: u32, to: u32) -> Result<(), VaultError> {
    match (from, to) {
        (1, 2) => Ok(()), // no-op migration: preserves all existing state
        _ => Err(VaultError::MigrationSkipped),
    }
}
