use crate::{errors::VaultError, events, storage};
use soroban_sdk::{Address, BytesN, Env};

pub fn set_admin(env: Env, new_admin: Address) -> Result<(), VaultError> {
    let current_admin = storage::get_admin(&env).ok_or(VaultError::InvalidInputs)?;
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

pub fn set_lending_pool(env: Env, lending_pool: Address) {
    let admin = storage::get_admin(&env).expect("not initialized");
    admin.require_auth();

    let old_pool = crate::storage::_get_lending_pool(&env);

    storage::set_lending_pool(&env, &lending_pool);

    events::LendingPoolUpdated {
        old_pool,
        new_pool: lending_pool,
    }
    .publish(&env);
}

pub fn set_oracle(env: Env, oracle: Address) {
    let admin = storage::get_admin(&env).expect("not initialized");
    admin.require_auth();

    let old_oracle = storage::get_oracle(&env);
    storage::set_oracle(&env, &oracle);

    events::OracleUpdated {
        old_oracle,
        new_oracle: oracle,
    }
    .publish(&env);
}

pub fn set_liquidation_engine(env: Env, engine: Address) {
    let admin = storage::get_admin(&env).expect("not initialized");
    admin.require_auth();

    let old_engine = storage::get_liquidation_engine(&env);
    storage::set_liquidation_engine(&env, &engine);

    events::LiquidationEngineUpdated {
        old_engine,
        new_engine: engine,
    }
    .publish(&env);
}

pub fn set_pool(env: Env, pool: Address) {
    let admin = storage::get_admin(&env).expect("not initialized");
    admin.require_auth();

    let old_pool = storage::get_pool(&env);
    storage::set_pool(&env, &pool);

    events::PoolUpdated {
        old_pool,
        new_pool: pool,
    }
    .publish(&env);
}

pub fn pause(env: Env) {
    let admin = storage::get_admin(&env).expect("not initialized");
    admin.require_auth();

    if storage::is_paused(&env) {
        soroban_sdk::panic_with_error!(&env, VaultError::AlreadyPaused);
    }

    storage::set_paused(&env, true);

    events::Paused { by: admin }.publish(&env);
}

pub fn unpause(env: Env) {
    let admin = storage::get_admin(&env).expect("not initialized");
    admin.require_auth();

    if !storage::is_paused(&env) {
        soroban_sdk::panic_with_error!(&env, VaultError::NotPaused);
    }

    storage::set_paused(&env, false);

    events::Unpaused { by: admin }.publish(&env);
}

pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
    let admin = storage::get_admin(&env).expect("not initialized");
    admin.require_auth();

    env.deployer()
        .update_current_contract_wasm(new_wasm_hash.clone());

    events::ContractUpgraded {
        old_hash: None,
        new_hash: new_wasm_hash,
    }
    .publish(&env);
}
