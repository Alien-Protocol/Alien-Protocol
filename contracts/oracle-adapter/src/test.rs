#![cfg(test)]

use crate::{OracleContract, OracleContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_get_price_returns_correct_data() {
    let env = Env::default();
    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let asset = Address::generate(&env);
    let price = 1_000_000i128;
    let timestamp = 1234567890u64;

    client.set_price(&asset, &price, &timestamp);

    let result = client.get_price(&asset);
    assert!(result.is_some());

    let price_data = result.unwrap();
    assert_eq!(price_data.price, price);
    assert_eq!(price_data.timestamp, timestamp);
}

#[test]
fn test_get_price_returns_none_for_unknown_asset() {
    let env = Env::default();
    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let unknown_asset = Address::generate(&env);

    let result = client.get_price(&unknown_asset);
    assert!(result.is_none());
}

#[test]
fn test_get_price_does_not_mutate_state() {
    let env = Env::default();
    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let asset = Address::generate(&env);
    let price = 2_000_000i128;
    let timestamp = 9876543210u64;

    client.set_price(&asset, &price, &timestamp);

    let result1 = client.get_price(&asset);
    let result2 = client.get_price(&asset);

    assert_eq!(result1, result2);
    assert!(result1.is_some());

    let data = result1.unwrap();
    assert_eq!(data.price, price);
    assert_eq!(data.timestamp, timestamp);
}

#[test]
fn test_get_price_multiple_assets() {
    let env = Env::default();
    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let asset1 = Address::generate(&env);
    let asset2 = Address::generate(&env);
    let price1 = 100_000i128;
    let price2 = 200_000i128;
    let timestamp1 = 1000u64;
    let timestamp2 = 2000u64;

    client.set_price(&asset1, &price1, &timestamp1);
    client.set_price(&asset2, &price2, &timestamp2);

    let result1 = client.get_price(&asset1);
    let result2 = client.get_price(&asset2);

    assert!(result1.is_some());
    assert!(result2.is_some());

    let data1 = result1.unwrap();
    let data2 = result2.unwrap();

    assert_eq!(data1.price, price1);
    assert_eq!(data1.timestamp, timestamp1);
    assert_eq!(data2.price, price2);
    assert_eq!(data2.timestamp, timestamp2);
}

#[test]
fn test_get_price_after_update() {
    let env = Env::default();
    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let asset = Address::generate(&env);
    let initial_price = 500_000i128;
    let updated_price = 750_000i128;
    let initial_timestamp = 5000u64;
    let updated_timestamp = 6000u64;

    client.set_price(&asset, &initial_price, &initial_timestamp);
    let initial_result = client.get_price(&asset);
    assert!(initial_result.is_some());
    assert_eq!(initial_result.as_ref().unwrap().price, initial_price);

    client.set_price(&asset, &updated_price, &updated_timestamp);
    let updated_result = client.get_price(&asset);
    assert!(updated_result.is_some());

    let updated_data = updated_result.unwrap();
    assert_eq!(updated_data.price, updated_price);
    assert_eq!(updated_data.timestamp, updated_timestamp);
}
