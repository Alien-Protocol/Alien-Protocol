#![cfg(test)]

use crate::{OracleContract, OracleContractClient, OracleError};
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, Env};

const STALENESS_THRESHOLD: u64 = 300;

fn setup() -> (Env, OracleContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &STALENESS_THRESHOLD);

    (env, client, admin)
}

#[test]
fn test_is_price_fresh_returns_true_for_recent_price() {
    let (env, client, _admin) = setup();
    let asset = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    client.set_price(&asset, &100, &900);

    assert!(client.is_price_fresh(&asset));
}

#[test]
fn test_is_price_fresh_returns_false_for_stale_price() {
    let (env, client, _admin) = setup();
    let asset = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    client.set_price(&asset, &100, &500);

    assert!(!client.is_price_fresh(&asset));
}

#[test]
fn test_is_price_fresh_returns_false_for_unknown_asset() {
    let (env, client, _admin) = setup();
    let unknown_asset = Address::generate(&env);

    assert!(!client.is_price_fresh(&unknown_asset));
}

#[test]
fn test_get_price_or_fail_success() {
    let (env, client, _admin) = setup();
    let asset = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    client.set_price(&asset, &12_345, &900);

    let data = client.get_price_or_fail(&asset);
    assert_eq!(data.price, 12_345);
    assert_eq!(data.timestamp, 900);
}

#[test]
fn test_get_price_or_fail_panics_on_missing_price() {
    let (env, client, _admin) = setup();
    let unknown_asset = Address::generate(&env);

    let result = client.try_get_price_or_fail(&unknown_asset);
    assert!(result.is_err());
    let err = result.err().unwrap().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(OracleError::PriceNotFound as u32)
    );
}

#[test]
fn test_get_price_or_fail_panics_on_stale_price() {
    let (env, client, _admin) = setup();
    let asset = Address::generate(&env);

    env.ledger().set_timestamp(1_000);
    client.set_price(&asset, &100, &500);

    let result = client.try_get_price_or_fail(&asset);
    assert!(result.is_err());
    let err = result.err().unwrap().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(OracleError::StalePrice as u32)
    );
}

#[test]
fn test_staleness_boundary_exact_threshold() {
    let (env, client, _admin) = setup();
    let asset = Address::generate(&env);

    client.set_price(&asset, &100, &100);
    env.ledger().set_timestamp(400);

    assert!(client.is_price_fresh(&asset));

    let data = client.get_price_or_fail(&asset);
    assert_eq!(data.price, 100);
    assert_eq!(data.timestamp, 100);
}

#[test]
fn test_staleness_boundary_one_second_over() {
    let (env, client, _admin) = setup();
    let asset = Address::generate(&env);

    client.set_price(&asset, &100, &100);
    env.ledger().set_timestamp(401);

    assert!(!client.is_price_fresh(&asset));

    let result = client.try_get_price_or_fail(&asset);
    assert!(result.is_err());
    let err = result.err().unwrap().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(OracleError::StalePrice as u32)
    );
}
