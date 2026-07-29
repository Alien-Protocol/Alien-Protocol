/// admin.rs — administrative operations for the collateral vault.
///
/// Every public function returns `Result<_, VaultError>`.  No `panic!`,
/// `expect`, or `unwrap` appears in production paths.
use crate::{config, errors::VaultError, events, storage};
use soroban_sdk::{Address, BytesN, Env};
use crate::{errors::VaultError, events, storage};
use soroban_sdk::{Address, Env};

/// Transfer administrative authority to `new_admin`.
///
/// Errors:
/// - `NotInitialized`  — vault has not been initialized.
/// - `AlreadyAdmin`    — `new_admin` is the same address as the current admin.
pub fn set_admin(env: Env, new_admin: Address) -> Result<(), VaultError> {
    let current_admin = config::require_admin(&env)?;
    current_admin.require_auth();

    if current_admin == new_admin {
        return Err(VaultError::AlreadyAdmin);
    }

    storage::set_admin(&env, &new_admin);

    events::AdminChanged {
        old_admin: current_admin,
        new_admin,
    }
    .publish(&env);

    Ok(())
}

/// Point the vault at a new lending-pool contract.
///
/// Errors:
/// - `NotInitialized` — vault has not been initialized.
pub fn set_lending_pool(env: Env, lending_pool: Address) -> Result<(), VaultError> {
    let admin = config::require_admin(&env)?;
    admin.require_auth();

    let old_pool = storage::_get_lending_pool(&env);
    storage::set_lending_pool(&env, &lending_pool);

    events::LendingPoolUpdated {
        old_pool,
        new_pool: lending_pool,
    }
    .publish(&env);

    Ok(())
}

/// Point the vault at a new oracle contract.
///
/// Errors:
/// - `NotInitialized` — vault has not been initialized.
pub fn set_oracle(env: Env, oracle: Address) -> Result<(), VaultError> {
    let admin = config::require_admin(&env)?;
    admin.require_auth();

    let old_oracle = storage::get_oracle(&env);
    storage::set_oracle(&env, &oracle);

    events::OracleUpdated {
        old_oracle,
        new_oracle: oracle,
    }
    .publish(&env);

    Ok(())
}

/// Register the liquidation-engine contract address.
///
/// Errors:
/// - `NotInitialized` — vault has not been initialized.
pub fn set_liquidation_engine(env: Env, engine: Address) -> Result<(), VaultError> {
    let admin = config::require_admin(&env)?;
    admin.require_auth();

    let old_engine = storage::get_liquidation_engine(&env);
    storage::set_liquidation_engine(&env, &engine);

    events::LiquidationEngineUpdated {
        old_engine,
        new_engine: engine,
    }
    .publish(&env);

    Ok(())
}

/// Register the lending pool address used for debt queries.
///
/// Errors:
/// - `NotInitialized` — vault has not been initialized.
pub fn set_pool(env: Env, pool: Address) -> Result<(), VaultError> {
    let admin = config::require_admin(&env)?;
    admin.require_auth();

    let old_pool = storage::get_pool(&env);
    storage::set_pool(&env, &pool);

    events::PoolUpdated {
        old_pool,
        new_pool: pool,
    }
    .publish(&env);

    Ok(())
}

/// Pause all state-mutating vault operations.
///
/// Errors:
/// - `NotInitialized` — vault has not been initialized.
/// - `AlreadyPaused`  — vault is already paused.
pub fn pause(env: Env) -> Result<(), VaultError> {
    let admin = config::require_admin(&env)?;
    admin.require_auth();

    if storage::is_paused(&env) {
        return Err(VaultError::AlreadyPaused);
    }

    storage::set_paused(&env, true);

    events::Paused { by: admin }.publish(&env);

    Ok(())
}

/// Resume vault operations after a pause.
///
/// Errors:
/// - `NotInitialized` — vault has not been initialized.
/// - `NotPaused`      — vault is not currently paused.
pub fn unpause(env: Env) -> Result<(), VaultError> {
    let admin = config::require_admin(&env)?;
    admin.require_auth();

    if !storage::is_paused(&env) {
        return Err(VaultError::NotPaused);
    }

    storage::set_paused(&env, false);

    events::Unpaused { by: admin }.publish(&env);

    Ok(())
}

/// Upgrade the contract WASM to `new_wasm_hash`.
///
/// Errors:
/// - `NotInitialized` — vault has not been initialized.
pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), VaultError> {
    let admin = config::require_admin(&env)?;
    admin.require_auth();

    env.deployer()
        .update_current_contract_wasm(new_wasm_hash.clone());

    events::ContractUpgraded {
        old_hash: None,
        new_hash: new_wasm_hash,
    }
    .publish(&env);

    Ok(())
}
