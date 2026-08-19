#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env, Symbol};

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
    let oracle = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_contract_id = token_contract.address();
    let token_client = token::Client::new(&env, &token_contract_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract_id);

    client.initialize(&admin, &vault, &oracle, &token_contract_id, &500);

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
fn test_supply_success_pulls_tokens_and_records_balance() {
    let (_env, client, _admin, user, _vault, _token_id, token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.supply(&user, &500);

    assert_eq!(token_client.balance(&user), 500);
    assert_eq!(token_client.balance(&client.address), 500);
    assert_eq!(client.get_user_supply(&user), 500);
    assert_eq!(client.get_total_supply(), 500);
}

#[test]
fn test_supply_zero_or_negative_fails() {
    let (_env, client, _admin, user, _vault, _token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);

    let res = client.try_supply(&user, &0);
    assert!(res.is_err());

    let res = client.try_supply(&user, &-100);
    assert!(res.is_err());
}

#[test]
fn test_supply_when_paused_fails() {
    let (env, client, _admin, user, _vault, _token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.pause_operation(&PauseFlag::Supply, &Symbol::new(&env, "test"));

    let res = client.try_supply(&user, &500);
    assert!(res.is_err());
}

#[test]
fn test_withdraw_liquidity_partial() {
    let (_env, client, _admin, user, _vault, _token_id, token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.supply(&user, &500);

    assert_eq!(client.get_user_supply(&user), 500);
    assert_eq!(client.get_available_liquidity(), 500);

    client.withdraw_liquidity(&user, &200);

    assert_eq!(token_client.balance(&user), 700);
    assert_eq!(token_client.balance(&client.address), 300);
    assert_eq!(client.get_user_supply(&user), 300);
    assert_eq!(client.get_total_supply(), 300);
}

#[test]
fn test_withdraw_liquidity_exceeds_balance_fails() {
    let (_env, client, _admin, user, _vault, _token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.supply(&user, &500);

    let res = client.try_withdraw_liquidity(&user, &600);
    assert!(res.is_err());
}

#[test]
fn test_withdraw_liquidity_when_paused_fails() {
    let (env, client, _admin, user, _vault, _token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.supply(&user, &500);

    client.pause_operation(&PauseFlag::WithdrawLiquidity, &Symbol::new(&env, "test"));

    let res = client.try_withdraw_liquidity(&user, &200);
    assert!(res.is_err());
}

#[test]
fn test_utilization_zero_when_nothing_borrowed() {
    let (_env, client, _admin, user, _vault, _token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.supply(&user, &500);

    assert_eq!(client.get_utilization_bps(), 0);
}
