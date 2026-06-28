#![cfg(test)]

use crate::tests::setup_env;
use crate::{OracleContract, OracleContractClient, OracleError};
use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, Env, Symbol, TryFromVal,
};

#[test]
fn test_set_admin_success() {
    let (env, client, admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);
}

#[test]
fn test_set_admin_non_admin_fails() {
    let (env, client, admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);
}

#[test]
fn test_set_admin_same_address_fails() {
    let (_env, client, admin) = setup_env();

    let result = client.try_set_admin(&admin);
    assert!(result.is_err());
    let err = result.err().unwrap().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(OracleError::AlreadyAdmin as u32)
    );
}

#[test]
fn test_set_admin_emits_event() {
    let (env, client, _admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);
    let event_symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(event_symbol, Symbol::new(&env, "admin_changed"));
}

#[test]
fn test_old_admin_cannot_set_price_after_transfer() {
    let (env, client, admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    let _ = env.auths();

    let asset = Address::generate(&env);
    client.set_price(&new_admin, &asset, &300, &4000);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, new_admin);
    assert_ne!(*auth_addr, admin);
}

#[test]
fn test_new_admin_can_set_price_after_transfer() {
    let (env, client, _admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    let asset = Address::generate(&env);
    client.set_price(&new_admin, &asset, &150, &2000);

    let price_data = client.get_price(&asset).unwrap();
    assert_eq!(price_data.price, 150);
    assert_eq!(price_data.timestamp, 2000);
}

#[test]
fn test_get_admin_returns_correct_address() {
    let (_env, client, admin) = setup_env();

    let result = client.get_admin();
    assert!(result.is_some());
    assert_eq!(result.unwrap(), admin);
}

#[test]
fn test_get_admin_before_init_returns_none() {
    let env = Env::default();
    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let result = client.get_admin();
    assert!(result.is_none());
}
