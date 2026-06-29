#![cfg(test)]

use crate::tests::setup_env;
use crate::OracleError;
use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, Symbol, TryFromVal,
};

#[test]
fn test_add_feeder_success() {
    let (env, client, _admin) = setup_env();

    let feeder = Address::generate(&env);
    client.add_authorized_feeder(&feeder);

    let asset = Address::generate(&env);
    client.set_price(&feeder, &asset, &100, &1000);

    let price_data = client.get_price(&asset).unwrap();
    assert_eq!(price_data.price, 100);
    assert_eq!(price_data.timestamp, 1000);
}

#[test]
fn test_add_feeder_duplicate_fails() {
    let (env, client, _admin) = setup_env();

    let feeder = Address::generate(&env);
    client.add_authorized_feeder(&feeder);

    let result = client.try_add_authorized_feeder(&feeder);
    assert!(result.is_err());
    let err = result.err().unwrap().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(OracleError::AlreadyAuthorized as u32)
    );
}

#[test]
fn test_add_feeder_non_admin_fails() {
    let (env, client, admin) = setup_env();

    let feeder = Address::generate(&env);
    client.add_authorized_feeder(&feeder);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);
}

#[test]
fn test_add_feeder_emits_event() {
    let (env, client, _admin) = setup_env();

    let feeder = Address::generate(&env);
    client.add_authorized_feeder(&feeder);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);
    let event_symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(event_symbol, Symbol::new(&env, "feeder_added"));
}

#[test]
fn test_remove_feeder_success() {
    let (env, client, _admin) = setup_env();

    let feeder = Address::generate(&env);
    client.add_authorized_feeder(&feeder);

    let asset = Address::generate(&env);
    client.set_price(&feeder, &asset, &100, &1000);

    client.remove_authorized_feeder(&feeder);

    let result = client.try_set_price(&feeder, &asset, &200, &2000);
    assert!(result.is_err());
    let err = result.err().unwrap().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(OracleError::Unauthorized as u32)
    );
}

#[test]
fn test_remove_feeder_not_found_fails() {
    let (env, client, _admin) = setup_env();

    let feeder = Address::generate(&env);
    let result = client.try_remove_authorized_feeder(&feeder);
    assert!(result.is_err());
    let err = result.err().unwrap().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(OracleError::FeederNotFound as u32)
    );
}

#[test]
fn test_remove_feeder_non_admin_fails() {
    let (env, client, admin) = setup_env();

    let feeder = Address::generate(&env);
    client.add_authorized_feeder(&feeder);

    let _ = env.auths();

    client.remove_authorized_feeder(&feeder);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);
}

#[test]
fn test_remove_feeder_emits_event() {
    let (env, client, _admin) = setup_env();

    let feeder = Address::generate(&env);
    client.add_authorized_feeder(&feeder);

    client.remove_authorized_feeder(&feeder);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);
    let event_symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(event_symbol, Symbol::new(&env, "feeder_removed"));
}

#[test]
fn test_is_authorized_feeder_true() {
    let (env, client, _admin) = setup_env();

    let feeder = Address::generate(&env);
    client.add_authorized_feeder(&feeder);

    let result = client.is_authorized_feeder(&feeder);
    assert!(result);
}

#[test]
fn test_is_authorized_feeder_false() {
    let (env, client, _admin) = setup_env();

    let feeder = Address::generate(&env);
    let result = client.is_authorized_feeder(&feeder);
    assert!(!result);
}

#[test]
fn test_multiple_feeders_can_set_price() {
    let (env, client, _admin) = setup_env();

    let feeder1 = Address::generate(&env);
    let feeder2 = Address::generate(&env);

    client.add_authorized_feeder(&feeder1);
    client.add_authorized_feeder(&feeder2);

    let asset1 = Address::generate(&env);
    let asset2 = Address::generate(&env);

    client.set_price(&feeder1, &asset1, &100, &1000);
    client.set_price(&feeder2, &asset2, &200, &2000);

    let price_data1 = client.get_price(&asset1).unwrap();
    assert_eq!(price_data1.price, 100);
    assert_eq!(price_data1.timestamp, 1000);

    let price_data2 = client.get_price(&asset2).unwrap();
    assert_eq!(price_data2.price, 200);
    assert_eq!(price_data2.timestamp, 2000);
}
