#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::{OracleContract, OracleContractClient};

fn setup() -> (Env, OracleContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    (env, client, admin)
}

#[test]
fn test_initialize_stores_admin_and_threshold() {
    let (_, client, admin) = setup();
    client.initialize(&admin, &3600);
    assert_eq!(client.get_admin(), Some(admin));
    assert_eq!(client.get_staleness_threshold(), Some(3600));
}

#[test]
#[should_panic(expected = "AlreadyInitialized")]
fn test_initialize_panics_if_called_twice() {
    let (_, client, admin) = setup();
    client.initialize(&admin, &3600);
    client.initialize(&admin, &3600);
}

#[test]
fn test_get_price_returns_none_before_set() {
    let (env, client, admin) = setup();
    client.initialize(&admin, &3600);
    let asset = Address::generate(&env);
    assert_eq!(client.get_price(&asset), None);
}

#[test]
fn test_set_and_get_price() {
    let (env, client, admin) = setup();
    client.initialize(&admin, &3600);
    let asset = Address::generate(&env);
    client.set_price(&asset, &1_500_000_000, &1_000_000);
    let data = client.get_price(&asset).unwrap();
    assert_eq!(data.price, 1_500_000_000);
    assert_eq!(data.timestamp, 1_000_000);
}

#[test]
fn test_set_price_overwrites_existing() {
    let (env, client, admin) = setup();
    client.initialize(&admin, &3600);
    let asset = Address::generate(&env);
    client.set_price(&asset, &100, &1);
    client.set_price(&asset, &200, &2);
    let data = client.get_price(&asset).unwrap();
    assert_eq!(data.price, 200);
    assert_eq!(data.timestamp, 2);
}

#[test]
fn test_multiple_assets_independent() {
    let (env, client, admin) = setup();
    client.initialize(&admin, &3600);
    let asset_a = Address::generate(&env);
    let asset_b = Address::generate(&env);
    client.set_price(&asset_a, &1_000, &10);
    client.set_price(&asset_b, &2_000, &20);
    assert_eq!(client.get_price(&asset_a).unwrap().price, 1_000);
    assert_eq!(client.get_price(&asset_b).unwrap().price, 2_000);
}
