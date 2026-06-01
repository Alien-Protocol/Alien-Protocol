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
        token_contract_id,
        token_client,
        token_admin_client,
    )
}

#[test]
fn test_add_asset_success() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();
    let other_token = Address::generate(&env);

    client.add_supported_asset(&other_token);
    assert!(client.is_supported_asset(&other_token));
}

#[test]
fn test_add_asset_duplicate_fails() {
    let (_env, client, _admin, _user, token_id, _token_client, _token_admin) = setup_env();

    let res = client.try_add_supported_asset(&token_id);
    assert!(res.is_err());
}

#[test]
fn test_remove_asset_success() {
    let (_env, client, _admin, _user, token_id, _token_client, _token_admin) = setup_env();

    assert!(client.is_supported_asset(&token_id));
    client.remove_supported_asset(&token_id);
    assert!(!client.is_supported_asset(&token_id));
}

#[test]
fn test_remove_asset_not_found_fails() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();
    let unknown_token = Address::generate(&env);

    let res = client.try_remove_supported_asset(&unknown_token);
    assert!(res.is_err());
}

#[test]
fn test_remove_asset_does_not_clear_existing_positions() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.remove_supported_asset(&token_id);

    assert_eq!(client.get_position_balance(&user, &token_id), 500);
}

#[test]
fn test_is_supported_asset_true() {
    let (_env, client, _admin, _user, token_id, _token_client, _token_admin) = setup_env();
    assert!(client.is_supported_asset(&token_id));
}

#[test]
fn test_is_supported_asset_false() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();
    let unknown_token = Address::generate(&env);
    assert!(!client.is_supported_asset(&unknown_token));
}

#[test]
fn test_get_all_positions_empty() {
    let (_env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();
    let positions = client.get_all_positions();
    assert!(positions.is_empty());
}

#[test]
fn test_get_all_positions_multiple_users() {
    let (env, client, _admin, user1, token_id, _token_client, token_admin) = setup_env();
    let user2 = Address::generate(&env);

    token_admin.mint(&user1, &1000);
    client.deposit(&user1, &token_id, &500);

    token_admin.mint(&user2, &1000);
    client.deposit(&user2, &token_id, &300);

    let positions = client.get_all_positions();
    assert_eq!(positions.len(), 2);

    let mut found_user1 = false;
    let mut found_user2 = false;
    for p in positions.iter() {
        if p.user == user1 {
            found_user1 = true;
        }
        if p.user == user2 {
            found_user2 = true;
        }
    }
    assert!(found_user1);
    assert!(found_user2);
}

#[test]
fn test_get_all_positions_excludes_withdrawn() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    assert_eq!(client.get_all_positions().len(), 1);

    client.withdraw(&user, &token_id, &500);

    assert_eq!(client.get_all_positions().len(), 0);
}
