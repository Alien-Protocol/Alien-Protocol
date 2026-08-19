#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env};

fn setup_env() -> (
    Env,
    PoolContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PoolContract, ());
    let client = PoolContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let vault = Address::generate(&env);
    let _oracle = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_contract_id = token_contract.address();
    let token_client = token::Client::new(&env, &token_contract_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract_id);

    (
        env,
        client,
        admin,
        user,
        vault,
        token_contract_id,
        token_client,
        token_admin_client,
    )
}

#[test]
fn test_initialize_success() {
    let (env, client, admin, _user, vault, token_id, _token_client, _token_admin) = setup_env();
    let oracle = Address::generate(&env);

    client.initialize(&admin, &vault, &oracle, &token_id, &500);

    assert_eq!(client.get_admin(), Some(admin.clone()));
    assert_eq!(client.get_vault(), Some(vault.clone()));
    assert_eq!(client.get_oracle(), Some(oracle.clone()));
    assert_eq!(client.get_borrow_asset(), Some(token_id.clone()));
    assert_eq!(client.get_interest_rate_bps(), 500);
}

#[test]
fn test_initialize_duplicate_fails() {
    let (env, client, admin, _user, vault, token_id, _token_client, _token_admin) = setup_env();
    let oracle = Address::generate(&env);

    client.initialize(&admin, &vault, &oracle, &token_id, &500);

    let res = client.try_initialize(&admin, &vault, &oracle, &token_id, &500);
    assert!(res.is_err());
}

#[test]
fn test_initialize_vault_equals_oracle_fails() {
    let (_env, client, admin, _user, vault, token_id, _token_client, _token_admin) = setup_env();

    let res = client.try_initialize(&admin, &vault, &vault, &token_id, &500);
    assert!(res.is_err());
}

#[test]
fn test_initialize_requires_admin_auth() {
    let env = Env::default();
    let contract_id = env.register(PoolContract, ());
    let client = PoolContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let vault = Address::generate(&env);
    let oracle = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_contract_id = token_contract.address();

    // No mock_all_auths - initialize should fail auth check
    let res = client.try_initialize(&admin, &vault, &oracle, &token_contract_id, &500);
    assert!(res.is_err());
}

#[test]
fn test_set_admin_success() {
    let (env, client, admin, _user, vault, token_id, _token_client, _token_admin) = setup_env();
    let oracle = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.initialize(&admin, &vault, &oracle, &token_id, &500);
    client.set_admin(&new_admin);

    assert_eq!(client.get_admin(), Some(new_admin));
}

#[test]
fn test_set_admin_same_address_fails() {
    let (env, client, admin, _user, vault, token_id, _token_client, _token_admin) = setup_env();
    let oracle = Address::generate(&env);

    client.initialize(&admin, &vault, &oracle, &token_id, &500);

    let res = client.try_set_admin(&admin);
    assert!(res.is_err());
}

#[test]
fn test_getters_match_initialize_args() {
    let (env, client, admin, _user, vault, token_id, _token_client, _token_admin) = setup_env();
    let oracle = Address::generate(&env);
    let interest_rate_bps = 500;

    client.initialize(&admin, &vault, &oracle, &token_id, &interest_rate_bps);

    assert_eq!(client.get_admin(), Some(admin.clone()));
    assert_eq!(client.get_vault(), Some(vault.clone()));
    assert_eq!(client.get_oracle(), Some(oracle.clone()));
    assert_eq!(client.get_borrow_asset(), Some(token_id.clone()));
    assert_eq!(client.get_interest_rate_bps(), interest_rate_bps);
}
