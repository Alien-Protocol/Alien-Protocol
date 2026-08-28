#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::{token, Address, Env, Symbol};

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let oracle = Address::generate(&env);
    let lending_pool = Address::generate(&env);
    let liquidation_engine = Address::generate(&env);

    client.initialize(&admin, &lending_pool, &oracle, &liquidation_engine);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_contract_id = token_contract.address();
    let token_client = token::Client::new(&env, &token_contract_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract_id);

    client.add_supported_asset(&token_contract_id);

    (
        env,
        client,
        admin,
        user,
        oracle,
        lending_pool,
        liquidation_engine,
        token_contract_id,
        token_client,
        token_admin_client,
    )
}

// ── Initialization tests ───────────────────────────────────────────────

#[test]
fn test_initialize_success() {
    let (
        _env,
        client,
        admin,
        _user,
        oracle,
        lending_pool,
        liquidation_engine,
        _token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    assert_eq!(client.get_admin(), Some(admin));
    assert_eq!(client.get_lending_pool(), Some(lending_pool));
    assert_eq!(client.get_oracle(), Some(oracle));
    assert_eq!(client.get_liquidation_engine(), Some(liquidation_engine));
}

#[test]
fn test_initialize_duplicate_fails() {
    let (
        env,
        client,
        _admin,
        _user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        _token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    let admin2 = Address::generate(&env);
    let pool2 = Address::generate(&env);
    let oracle2 = Address::generate(&env);
    let engine2 = Address::generate(&env);

    let result = client.try_initialize(&admin2, &pool2, &oracle2, &engine2);
    assert_eq!(result, Err(Ok(VaultError::AlreadyInitialized)));
}

#[test]
fn test_initialize_pool_equals_oracle_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let same_address = Address::generate(&env);
    let engine = Address::generate(&env);

    // lending_pool == oracle should be rejected
    let result = client.try_initialize(&admin, &same_address, &same_address, &engine);
    assert_eq!(result, Err(Ok(VaultError::InvalidAddress)));
}

#[test]
fn test_initialize_sets_paused_false() {
    let (
        _env,
        client,
        _admin,
        _user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        _token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    let res = client.try_unpause_operation(&PauseFlag::Deposit);
    assert_eq!(res, Err(Ok(VaultError::NotPaused)));
}

#[test]
fn test_set_lending_pool_oracle_collision_fails() {
    let (
        _env,
        client,
        _admin,
        _user,
        oracle,
        _lending_pool,
        _liquidation_engine,
        _token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    // Setting lending_pool equal to current oracle should fail
    let res = client.try_set_lending_pool(&oracle);
    assert_eq!(res, Err(Ok(VaultError::InvalidAddress)));
}

#[test]
fn test_set_oracle_lending_pool_collision_fails() {
    let (
        _env,
        client,
        _admin,
        _user,
        _oracle,
        lending_pool,
        _liquidation_engine,
        _token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    // Setting oracle equal to current lending_pool should fail
    let res = client.try_set_oracle(&lending_pool);
    assert_eq!(res, Err(Ok(VaultError::InvalidAddress)));
}

// ── Admin transfer tests ───────────────────────────────────────────────

#[test]
fn test_set_admin_success() {
    let (
        env,
        client,
        _admin,
        _user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        _token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    assert_eq!(client.get_admin(), Some(new_admin));
}

#[test]
fn test_set_admin_non_admin_fails() {
    let (
        env,
        client,
        admin,
        _user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        _token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    // Assert that it was the admin address that was required to authorize the set_admin call
    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);
}

#[test]
fn test_set_admin_emits_event() {
    let (
        env,
        client,
        _admin,
        _user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        _token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);
    use soroban_sdk::TryFromVal;
    let event_symbol =
        soroban_sdk::Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(
        event_symbol,
        soroban_sdk::Symbol::new(&env, "admin_changed")
    );
}

#[test]
fn test_old_admin_cannot_act_after_transfer() {
    let (
        env,
        client,
        admin,
        _user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        _token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    // Old admin tries to pause - but contract requires auth from the admin in storage, which is now new_admin.
    // Under mock_all_auths, require_auth verifies new_admin.
    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "test"));

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, new_admin);
    assert_ne!(*auth_addr, admin);
}

// ── Pause / unpause tests ──────────────────────────────────────────────

#[test]
fn test_pause_success() {
    let (
        env,
        client,
        _admin,
        _user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        _token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "test"));
    let paused = env.as_contract(&client.address, || {
        storage::is_operation_paused(&env, &PauseFlag::Deposit)
    });
    assert!(paused);
}

#[test]
fn test_pause_blocks_deposit() {
    let (
        env,
        client,
        _admin,
        user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        token_id,
        _token_client,
        token_admin,
    ) = setup_env();

    token_admin.mint(&user, &1000);
    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "test"));

    let res = client.try_deposit(&user, &token_id, &500);
    assert!(res.is_err());
}

#[test]
fn test_pause_blocks_withdraw() {
    let (
        env,
        client,
        _admin,
        user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        token_id,
        _token_client,
        token_admin,
    ) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.pause_operation(&PauseFlag::Withdraw, &Symbol::new(&env, "test"));

    let res = client.try_withdraw(&user, &token_id, &100);
    assert!(res.is_err());
}

#[test]
fn test_double_pause_fails() {
    let (
        env,
        client,
        _admin,
        _user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        _token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "first"));
    let res = client.try_pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "second"));
    assert!(res.is_err());
}

#[test]
fn test_unpause_success() {
    let (
        env,
        client,
        _admin,
        user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        token_id,
        token_client,
        token_admin,
    ) = setup_env();

    token_admin.mint(&user, &1000);
    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "test"));
    client.unpause_operation(&PauseFlag::Deposit);

    // Deposit should work again
    client.deposit(&user, &token_id, &500);
    assert_eq!(token_client.balance(&user), 500);
}

#[test]
fn test_unpause_when_not_paused_fails() {
    let (
        _env,
        client,
        _admin,
        _user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        _token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    let res = client.try_unpause_operation(&PauseFlag::Deposit);
    assert!(res.is_err());
}

#[test]
fn test_unpause_emits_event() {
    let (
        env,
        client,
        _admin,
        _user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        _token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "test"));
    client.unpause_operation(&PauseFlag::Deposit);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);
    use soroban_sdk::TryFromVal;
    let event_symbol =
        soroban_sdk::Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(
        event_symbol,
        soroban_sdk::Symbol::new(&env, "operation_unpaused")
    );
}

// ── Asset management tests ─────────────────────────────────────────────

#[test]
fn test_remove_supported_asset_success() {
    let (
        _env,
        client,
        _admin,
        _user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    assert!(client.is_supported_asset(&token_id));
    client.remove_supported_asset(&token_id);
    assert!(!client.is_supported_asset(&token_id));
}

#[test]
fn test_remove_supported_asset_non_existent_fails() {
    let (
        env,
        client,
        _admin,
        _user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        _token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    let fake_asset = Address::generate(&env);
    let res = client.try_remove_supported_asset(&fake_asset);
    assert!(res.is_err());
}

#[test]
fn test_remove_supported_asset_blocks_deposit() {
    let (
        _env,
        client,
        _admin,
        user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        token_id,
        _token_client,
        token_admin,
    ) = setup_env();

    token_admin.mint(&user, &1000);
    client.remove_supported_asset(&token_id);

    let res = client.try_deposit(&user, &token_id, &500);
    assert!(res.is_err());
}

#[test]
fn test_remove_supported_asset_with_open_position_fails() {
    let (
        _env,
        client,
        _admin,
        user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        token_id,
        _token_client,
        token_admin,
    ) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_remove_supported_asset(&token_id);
    assert_eq!(res, Err(Ok(VaultError::AssetHasOpenPositions)));
}

#[test]
fn test_remove_supported_asset_emits_event() {
    let (
        env,
        client,
        _admin,
        _user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    client.remove_supported_asset(&token_id);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);
    use soroban_sdk::TryFromVal;
    let event_symbol =
        soroban_sdk::Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(
        event_symbol,
        soroban_sdk::Symbol::new(&env, "asset_removed")
    );
}

#[test]
fn test_set_admin_same_address() {
    let (
        _env,
        client,
        admin,
        _user,
        _oracle,
        _lending_pool,
        _liquidation_engine,
        _token_id,
        _token_client,
        _token_admin,
    ) = setup_env();

    let result = client.try_set_admin(&admin);
    assert_eq!(result, Err(Ok(VaultError::AlreadyAdmin)));
}

// ── Uninitialized admin calls return Err (not `expect` panic) ──────────

#[test]
fn test_uninitialized_admin_setters_return_err() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let some_addr = Address::generate(&env);

    assert_eq!(
        client.try_set_lending_pool(&some_addr),
        Err(Ok(VaultError::NotInitialized))
    );
    assert_eq!(
        client.try_set_oracle(&some_addr),
        Err(Ok(VaultError::NotInitialized))
    );
    assert_eq!(
        client.try_set_liquidation_engine(&some_addr),
        Err(Ok(VaultError::NotInitialized))
    );
    assert_eq!(
        client.try_pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "reason")),
        Err(Ok(VaultError::NotInitialized))
    );
    assert_eq!(
        client.try_unpause_operation(&PauseFlag::Deposit),
        Err(Ok(VaultError::NotInitialized))
    );

    // Nothing should have been set: the admin is still uninitialized
    assert_eq!(client.get_admin(), None);
}

#[test]
fn test_uninitialized_set_admin_returns_err() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    // set_admin already behaves this way and the issue requires it remain unchanged.
    // Calling it on an uninitialized contract should return InvalidInputs.
    let some_addr = Address::generate(&env);
    assert_eq!(
        client.try_set_admin(&some_addr),
        Err(Ok(VaultError::InvalidInputs))
    );
}
