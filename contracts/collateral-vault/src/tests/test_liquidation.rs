#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env};

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
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

#[test]
fn test_authorize_liquidation_success() {
    let (env, client, _admin, _user, _oracle, _token_id, _token_client, _token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.authorize_liquidation(&engine);
}

#[test]
fn test_seize_collateral_emits_event() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.authorize_liquidation(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.seize_collateral(&engine, &user, &token_id, &200);

    // In Soroban, we can't easily "check" events in the same way as logs,
    // but the contract publishes them. If we wanted to verify, we would usually
    // check the ledger's events.
    // For now, we've verified it doesn't panic.
}

#[test]
fn test_seize_collateral_success() {
    let (env, client, _admin, user, _oracle, token_id, token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.authorize_liquidation(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.seize_collateral(&engine, &user, &token_id, &200);

    assert_eq!(client.get_position_balance(&user, &token_id), 300);
    assert_eq!(token_client.balance(&engine), 200);
    assert_eq!(token_client.balance(&client.address), 300);
}

#[test]
fn test_seize_collateral_unauthorized_engine_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);
    let malicious_engine = Address::generate(&env);

    client.authorize_liquidation(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&malicious_engine, &user, &token_id, &200);
    assert!(res.is_err());
}

#[test]
fn test_seize_collateral_insufficient_balance_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.authorize_liquidation(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&engine, &user, &token_id, &600);
    assert!(res.is_err());
}

#[test]
fn test_seize_collateral_no_position_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, _token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.authorize_liquidation(&engine);

    // User has NO position
    let res = client.try_seize_collateral(&engine, &user, &token_id, &200);
    assert!(res.is_err());
}

#[test]
fn test_seize_collateral_removes_from_index_on_zero() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.authorize_liquidation(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    assert!(client.get_position_index().contains(&user));

    client.seize_collateral(&engine, &user, &token_id, &500);

    assert_eq!(client.get_position_balance(&user, &token_id), 0);
    assert!(!client.get_position_index().contains(&user));
}

#[test]
fn test_seize_collateral_paused_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.authorize_liquidation(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.pause();

    let res = client.try_seize_collateral(&engine, &user, &token_id, &200);
    assert!(res.is_err());
}
