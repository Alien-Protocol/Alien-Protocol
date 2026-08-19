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
fn test_pause_and_unpause_borrow() {
    let (env, client, _admin, _user, _vault, _token_id, _token_client, _token_admin) = setup_env();

    client.pause_operation(&PauseFlag::Borrow, &Symbol::new(&env, "test"));

    let res = client.try_unpause_operation(&PauseFlag::Borrow);
    assert!(res.is_ok());
}

#[test]
fn test_double_pause_fails() {
    let (env, client, _admin, _user, _vault, _token_id, _token_client, _token_admin) = setup_env();

    client.pause_operation(&PauseFlag::Borrow, &Symbol::new(&env, "test"));

    let res = client.try_pause_operation(&PauseFlag::Borrow, &Symbol::new(&env, "test"));
    assert!(res.is_err());
}

#[test]
fn test_unpause_when_not_paused_fails() {
    let (_env, client, _admin, _user, _vault, _token_id, _token_client, _token_admin) = setup_env();

    let res = client.try_unpause_operation(&PauseFlag::Borrow);
    assert!(res.is_err());
}
