#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env};

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
    Address, // admin
    Address, // user
    Address, // oracle
    Address, // token_id
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

    client.initialize(&admin, &oracle);

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
        token_contract_id,
        token_client,
        token_admin_client,
    )
}

// ── authorize_liquidation ───────────────────────────────────────────────────

#[test]
fn test_authorize_liquidation_engine_can_be_set() {
    let (env, client, _admin, _user, _oracle, _token_id, _token_client, _token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);
}

// ── seize_collateral ────────────────────────────────────────────────────────

#[test]
fn test_seize_collateral_success() {
    let (env, client, _admin, user, _oracle, token_id, token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.seize_collateral(&engine, &user, &token_id, &200);

    assert_eq!(client.get_position_balance(&user, &token_id), 300);
    assert_eq!(token_client.balance(&engine), 200);
    assert_eq!(token_client.balance(&client.address), 300);
}

#[test]
fn test_seize_collateral_emits_event() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    // Should not return an error.
    client.seize_collateral(&engine, &user, &token_id, &200);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);
    use soroban_sdk::TryFromVal;
    let sym =
        soroban_sdk::Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(sym, soroban_sdk::Symbol::new(&env, "collateral_seized"));
}

#[test]
fn test_seize_collateral_removes_from_index_on_zero() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    assert!(client.get_position_index().contains(&user));

    client.seize_collateral(&engine, &user, &token_id, &500);

    assert_eq!(client.get_position_balance(&user, &token_id), 0);
    assert!(!client.get_position_index().contains(&user));
}

// ── failure paths ────────────────────────────────────────────────────────────

#[test]
fn test_seize_collateral_no_engine_configured_fails_with_liquidation_engine_not_set() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    // Intentionally do NOT call set_liquidation_engine.
    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let err = client
        .try_seize_collateral(&engine, &user, &token_id, &200)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, VaultError::LiquidationEngineNotSet);
}

#[test]
fn test_seize_collateral_unauthorized_engine_fails_with_unauthorized() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);
    let malicious_engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let err = client
        .try_seize_collateral(&malicious_engine, &user, &token_id, &200)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, VaultError::Unauthorized);
}

#[test]
fn test_seize_collateral_insufficient_balance_fails_with_insufficient_balance() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let err = client
        .try_seize_collateral(&engine, &user, &token_id, &600)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, VaultError::InsufficientBalance);
}

#[test]
fn test_seize_collateral_no_position_fails_with_no_position() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, _token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);
    // User has NO position.
    let err = client
        .try_seize_collateral(&engine, &user, &token_id, &200)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, VaultError::NoPosition);
}

#[test]
fn test_seize_collateral_paused_fails_with_vault_paused() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);
    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);
    client.pause();

    let err = client
        .try_seize_collateral(&engine, &user, &token_id, &200)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, VaultError::VaultPaused);
}
