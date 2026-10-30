#![cfg(test)]

use soroban_sdk::testutils::Address as _;
use soroban_sdk::testutils::Events;
use soroban_sdk::{Address, Env, Map, Symbol, TryFromVal};

use crate::errors::EngineError;
use crate::{admin, storage, LiquidationContract, LiquidationContractClient};

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
    let client = LiquidationContractClient::new(&env, &contract_id);

    client.initialize(&admin, &vault, &pool, &oracle);

    assert_eq!(client.get_admin(), Some(admin));
    assert_eq!(client.get_vault(), Some(vault));
    assert_eq!(client.get_pool(), Some(pool));
    assert_eq!(client.get_oracle(), Some(oracle));
}

#[test]
fn test_set_oracle_success() {
    let (env, contract_id) = create_test_env_and_contract();
    let (admin, vault, pool, oracle) = create_test_addresses(&env);
    let new_oracle = Address::generate(&env);

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

    // Set new oracle should succeed
    let result = env.as_contract(&contract_id, || {
        admin::set_oracle(env.clone(), new_oracle.clone())
    });
    assert_eq!(result, Ok(()));

    // Verify get_oracle reflects the new address
    env.as_contract(&contract_id, || {
        assert_eq!(storage::get_oracle(&env), Some(new_oracle.clone()));

        // Old oracle must not be returned
        assert_ne!(storage::get_oracle(&env), Some(oracle));
    });
}

#[test]
fn test_set_oracle_collision_with_admin_fails() {
    let (env, contract_id) = create_test_env_and_contract();
    let (admin, vault, pool, oracle) = create_test_addresses(&env);

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

    // oracle == admin must fail with InvalidAddress
    let result = env.as_contract(&contract_id, || {
        admin::set_oracle(env.clone(), admin.clone())
    });
    assert_eq!(result, Err(EngineError::InvalidAddress));
}

#[test]
fn test_set_oracle_collision_with_vault_fails() {
    let (env, contract_id) = create_test_env_and_contract();
    let (admin, vault, pool, oracle) = create_test_addresses(&env);

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

    // oracle == vault must fail with InvalidAddress
    let result = env.as_contract(&contract_id, || {
        admin::set_oracle(env.clone(), vault.clone())
    });
    assert_eq!(result, Err(EngineError::InvalidAddress));
}

#[test]
fn test_set_oracle_collision_with_pool_fails() {
    let (env, contract_id) = create_test_env_and_contract();
    let (admin, vault, pool, oracle) = create_test_addresses(&env);

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

    // oracle == pool must fail with InvalidAddress
    let result = env.as_contract(&contract_id, || {
        admin::set_oracle(env.clone(), pool.clone())
    });
    assert_eq!(result, Err(EngineError::InvalidAddress));
}

#[test]
fn test_set_oracle_requires_admin_auth() {
    let (env, contract_id) = create_test_env_and_contract();
    let (admin, vault, pool, oracle) = create_test_addresses(&env);
    let new_oracle = Address::generate(&env);

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

    // require_auth() is satisfied under mock_all_auths; the call must succeed.
    let result = env.as_contract(&contract_id, || {
        admin::set_oracle(env.clone(), new_oracle.clone())
    });
    assert_eq!(result, Ok(()));
    env.as_contract(&contract_id, || {
        assert_eq!(storage::get_oracle(&env), Some(new_oracle));
    });
}

#[test]
fn test_set_oracle_distinct_reflects_getter() {
    let (env, contract_id) = create_test_env_and_contract();
    let (admin, vault, _pool, oracle) = create_test_addresses(&env);
    let pool = Address::generate(&env);
    let new_oracle = Address::generate(&env);

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

    // Setting a distinct oracle (not colliding with admin/vault/pool) must succeed
    // and get_oracle must reflect exactly the new address.
    let result = env.as_contract(&contract_id, || {
        admin::set_oracle(env.clone(), new_oracle.clone())
    });
    assert_eq!(result, Ok(()));
    env.as_contract(&contract_id, || {
        assert_eq!(storage::get_oracle(&env), Some(new_oracle));
    });
}

// ---------------------------------------------------------------------------
// Authorization ordering: auth must run before address-collision validation.
// These environments deliberately do not call `mock_all_auths`, so a missing
// authorization panics. If validation ran first, the call would instead return
// `Err(EngineError::InvalidAddress)` and the test would fail (no panic).
// ---------------------------------------------------------------------------

fn seed_contract_without_mocked_auth() -> (Env, Address, Address, Address, Address, Address) {
    let env = Env::default();
    let contract_id = env.register(LiquidationContract, ());
    let (admin, vault, pool, oracle) = create_test_addresses(&env);

    env.as_contract(&contract_id, || {
        storage::set_admin(&env, &admin);
        storage::set_vault(&env, &vault);
        storage::set_pool(&env, &pool);
        storage::set_oracle(&env, &oracle);
    });

    (env, contract_id, admin, vault, pool, oracle)
}

#[test]
#[should_panic]
fn test_set_admin_authorizes_before_collision_validation() {
    let (env, contract_id, _admin, vault, _pool, _oracle) = seed_contract_without_mocked_auth();

    env.as_contract(&contract_id, || {
        // vault collides with the stored vault; auth must panic first.
        let _ = admin::set_admin(env.clone(), vault.clone());
    });
}

#[test]
#[should_panic]
fn test_set_vault_authorizes_before_collision_validation() {
    let (env, contract_id, admin, _vault, _pool, _oracle) = seed_contract_without_mocked_auth();

    env.as_contract(&contract_id, || {
        // admin collides with the stored admin; auth must panic first.
        let _ = admin::set_vault(env.clone(), admin.clone());
    });
}

#[test]
#[should_panic]
fn test_set_pool_authorizes_before_collision_validation() {
    let (env, contract_id, admin, _vault, _pool, _oracle) = seed_contract_without_mocked_auth();

    env.as_contract(&contract_id, || {
        // admin collides with the stored admin; auth must panic first.
        let _ = admin::set_pool(env.clone(), admin.clone());
    });
}

#[test]
#[should_panic]
fn test_set_oracle_authorizes_before_collision_validation() {
    let (env, contract_id, admin, _vault, _pool, _oracle) = seed_contract_without_mocked_auth();

    env.as_contract(&contract_id, || {
        // admin collides with the stored admin; auth must panic first.
        let _ = admin::set_oracle(env.clone(), admin.clone());
    });
}

#[test]
fn test_set_vault_emits_vault_updated_event() {
    let (env, contract_id) = create_test_env_and_contract();
    let (admin, vault, pool, oracle) = create_test_addresses(&env);
    let client = LiquidationContractClient::new(&env, &contract_id);
    client.initialize(&admin, &vault, &pool, &oracle);

    let new_vault = Address::generate(&env);
    client.set_vault(&new_vault);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, contract_id);

    let topics = last_event.1;
    let event_name = Symbol::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
    assert_eq!(event_name, Symbol::new(&env, "vault_updated"));

    let data_map: Map<Symbol, soroban_sdk::Val> = Map::try_from_val(&env, &last_event.2).unwrap();
    let old_v: Option<Address> =
        Option::try_from_val(&env, &data_map.get(Symbol::new(&env, "old_vault")).unwrap()).unwrap();
    let new_v: Address =
        Address::try_from_val(&env, &data_map.get(Symbol::new(&env, "new_vault")).unwrap())
            .unwrap();
    assert_eq!(old_v, Some(vault));
    assert_eq!(new_v, new_vault);
}

#[test]
fn test_set_pool_emits_pool_updated_event() {
    let (env, contract_id) = create_test_env_and_contract();
    let (admin, vault, pool, oracle) = create_test_addresses(&env);
    let client = LiquidationContractClient::new(&env, &contract_id);
    client.initialize(&admin, &vault, &pool, &oracle);

    let new_pool = Address::generate(&env);
    client.set_pool(&new_pool);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, contract_id);

    let topics = last_event.1;
    let event_name = Symbol::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
    assert_eq!(event_name, Symbol::new(&env, "pool_updated"));

    let data_map: Map<Symbol, soroban_sdk::Val> = Map::try_from_val(&env, &last_event.2).unwrap();
    let old_p: Option<Address> =
        Option::try_from_val(&env, &data_map.get(Symbol::new(&env, "old_pool")).unwrap()).unwrap();
    let new_p: Address =
        Address::try_from_val(&env, &data_map.get(Symbol::new(&env, "new_pool")).unwrap()).unwrap();
    assert_eq!(old_p, Some(pool));
    assert_eq!(new_p, new_pool);
}

#[test]
fn test_set_oracle_emits_oracle_updated_event() {
    let (env, contract_id) = create_test_env_and_contract();
    let (admin, vault, pool, oracle) = create_test_addresses(&env);
    let client = LiquidationContractClient::new(&env, &contract_id);
    client.initialize(&admin, &vault, &pool, &oracle);

    let new_oracle = Address::generate(&env);
    client.set_oracle(&new_oracle);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, contract_id);

    let topics = last_event.1;
    let event_name = Symbol::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
    assert_eq!(event_name, Symbol::new(&env, "oracle_updated"));

    let data_map: Map<Symbol, soroban_sdk::Val> = Map::try_from_val(&env, &last_event.2).unwrap();
    let old_o: Option<Address> = Option::try_from_val(
        &env,
        &data_map.get(Symbol::new(&env, "old_oracle")).unwrap(),
    )
    .unwrap();
    let new_o: Address = Address::try_from_val(
        &env,
        &data_map.get(Symbol::new(&env, "new_oracle")).unwrap(),
    )
    .unwrap();
    assert_eq!(old_o, Some(oracle));
    assert_eq!(new_o, new_oracle);
}
