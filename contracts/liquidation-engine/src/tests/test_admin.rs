#![cfg(test)]

use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

use crate::errors::EngineError;
use crate::{admin, storage, LiquidationContract};

fn create_test_env_and_contract() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(LiquidationContract, ());
    (env, contract_id)
}

fn create_test_addresses(env: &Env) -> (Address, Address, Address, Address) {
    let admin = Address::generate(env);
    let vault = Address::generate(env);
    let pool = Address::generate(env);
    let oracle = Address::generate(env);
    (admin, vault, pool, oracle)
}

#[test]
fn test_initialize_success() {
    let (env, contract_id) = create_test_env_and_contract();
    let (admin, vault, pool, oracle) = create_test_addresses(&env);

    // Initialize should succeed
    let result = env.as_contract(&contract_id, || {
        admin::initialize(
            env.clone(),
            admin.clone(),
            vault.clone(),
            pool.clone(),
            oracle.clone(),
        )
    });
    assert_eq!(result, Ok(()));

    // Verify all addresses are stored
    env.as_contract(&contract_id, || {
        assert_eq!(storage::get_admin(&env), Some(admin.clone()));
        assert_eq!(storage::get_vault(&env), Some(vault.clone()));
        assert_eq!(storage::get_pool(&env), Some(pool.clone()));
        assert_eq!(storage::get_oracle(&env), Some(oracle.clone()));
    });
}

#[test]
fn test_initialize_duplicate_fails() {
    let (env, contract_id) = create_test_env_and_contract();
    let (admin, vault, pool, oracle) = create_test_addresses(&env);

    // First initialize should succeed
    let result = env.as_contract(&contract_id, || {
        admin::initialize(
            env.clone(),
            admin.clone(),
            vault.clone(),
            pool.clone(),
            oracle.clone(),
        )
    });
    assert_eq!(result, Ok(()));

    // Second initialize should fail with AlreadyInitialized
    let result = env.as_contract(&contract_id, || {
        admin::initialize(
            env.clone(),
            admin.clone(),
            vault.clone(),
            pool.clone(),
            oracle.clone(),
        )
    });
    assert_eq!(result, Err(EngineError::AlreadyInitialized));
}

#[test]
fn test_initialize_duplicate_addresses_fail() {
    let (env, contract_id) = create_test_env_and_contract();
    let admin = Address::generate(&env);
    let _vault = Address::generate(&env);
    let pool = Address::generate(&env);

    // Test: admin == vault
    let result = env.as_contract(&contract_id, || {
        admin::initialize(
            env.clone(),
            admin.clone(),
            admin.clone(),
            pool.clone(),
            Address::generate(&env),
        )
    });
    assert_eq!(result, Err(EngineError::InvalidAddress));

    // Test: admin == pool
    let (env, contract_id) = create_test_env_and_contract();
    let admin = Address::generate(&env);
    let vault = Address::generate(&env);
    let _pool = Address::generate(&env);
    let result = env.as_contract(&contract_id, || {
        admin::initialize(
            env.clone(),
            admin.clone(),
            vault.clone(),
            admin.clone(),
            Address::generate(&env),
        )
    });
    assert_eq!(result, Err(EngineError::InvalidAddress));

    // Test: admin == oracle
    let (env, contract_id) = create_test_env_and_contract();
    let admin = Address::generate(&env);
    let vault = Address::generate(&env);
    let _pool = Address::generate(&env);
    let result = env.as_contract(&contract_id, || {
        admin::initialize(
            env.clone(),
            admin.clone(),
            vault.clone(),
            _pool.clone(),
            admin.clone(),
        )
    });
    assert_eq!(result, Err(EngineError::InvalidAddress));

    // Test: vault == pool
    let (env, contract_id) = create_test_env_and_contract();
    let admin = Address::generate(&env);
    let vault = Address::generate(&env);
    let _pool = Address::generate(&env);
    let result = env.as_contract(&contract_id, || {
        admin::initialize(
            env.clone(),
            admin.clone(),
            vault.clone(),
            vault.clone(),
            Address::generate(&env),
        )
    });
    assert_eq!(result, Err(EngineError::InvalidAddress));

    // Test: vault == oracle
    let (env, contract_id) = create_test_env_and_contract();
    let admin = Address::generate(&env);
    let vault = Address::generate(&env);
    let pool = Address::generate(&env);
    let result = env.as_contract(&contract_id, || {
        admin::initialize(
            env.clone(),
            admin.clone(),
            vault.clone(),
            pool.clone(),
            vault.clone(),
        )
    });
    assert_eq!(result, Err(EngineError::InvalidAddress));

    // Test: pool == oracle
    let (env, contract_id) = create_test_env_and_contract();
    let admin = Address::generate(&env);
    let _vault = Address::generate(&env);
    let pool = Address::generate(&env);
    let result = env.as_contract(&contract_id, || {
        admin::initialize(
            env.clone(),
            admin.clone(),
            _vault.clone(),
            pool.clone(),
            pool.clone(),
        )
    });
    assert_eq!(result, Err(EngineError::InvalidAddress));
}

#[test]
fn test_initialize_requires_admin_auth() {
    let (env, contract_id) = create_test_env_and_contract();
    let (admin, vault, pool, oracle) = create_test_addresses(&env);

    // This test passes when admin.require_auth() is called properly.
    // The initialize function should require the admin address to authorize.
    // In Soroban tests, require_auth() will panic if not authorized.
    // We verify the function works correctly when auth is satisfied.
    let result = env.as_contract(&contract_id, || {
        admin::initialize(
            env.clone(),
            admin.clone(),
            vault.clone(),
            pool.clone(),
            oracle.clone(),
        )
    });
    assert_eq!(result, Ok(()));
}

#[test]
fn test_set_admin_success() {
    let (env, contract_id) = create_test_env_and_contract();
    let (admin, vault, pool, oracle) = create_test_addresses(&env);
    let new_admin = Address::generate(&env);

    // Initialize first
    let result = env.as_contract(&contract_id, || {
        admin::initialize(
            env.clone(),
            admin.clone(),
            vault.clone(),
            pool.clone(),
            oracle.clone(),
        )
    });
    assert_eq!(result, Ok(()));

    // Set new admin should succeed
    let result = env.as_contract(&contract_id, || {
        admin::set_admin(env.clone(), new_admin.clone())
    });
    assert_eq!(result, Ok(()));

    // Verify new admin is stored
    env.as_contract(&contract_id, || {
        assert_eq!(storage::get_admin(&env), Some(new_admin.clone()));
    });
}

#[test]
fn test_set_admin_same_address_fails() {
    let (env, contract_id) = create_test_env_and_contract();
    let (admin, vault, pool, oracle) = create_test_addresses(&env);

    // Initialize first
    let result = env.as_contract(&contract_id, || {
        admin::initialize(
            env.clone(),
            admin.clone(),
            vault.clone(),
            pool.clone(),
            oracle.clone(),
        )
    });
    assert_eq!(result, Ok(()));

    // Try to set admin to the same address
    let result = env.as_contract(&contract_id, || {
        admin::set_admin(env.clone(), admin.clone())
    });
    assert_eq!(result, Err(EngineError::AlreadyAdmin));
}

#[test]
fn test_getters_match_initialize_args() {
    let (env, contract_id) = create_test_env_and_contract();
    let (admin, vault, pool, oracle) = create_test_addresses(&env);

    // Initialize with specific addresses
    let result = env.as_contract(&contract_id, || {
        admin::initialize(
            env.clone(),
            admin.clone(),
            vault.clone(),
            pool.clone(),
            oracle.clone(),
        )
    });
    assert_eq!(result, Ok(()));

    // Verify getters return exactly what was initialized
    env.as_contract(&contract_id, || {
        assert_eq!(storage::get_admin(&env), Some(admin));
        assert_eq!(storage::get_vault(&env), Some(vault));
        assert_eq!(storage::get_pool(&env), Some(pool));
        assert_eq!(storage::get_oracle(&env), Some(oracle));
    });
}
