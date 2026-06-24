#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Events;
use soroban_sdk::{Symbol, TryFromVal};

#[test]
fn test_pause_success() {
    let (env, client, _admin) = setup_env();
    client.pause();
    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);
    let event_symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(event_symbol, Symbol::new(&env, "paused"));
}

#[test]
fn test_pause_blocks_set_price() {
    let (_env, client, _admin) = setup_env();
    let asset = Address::generate(&_env);
    client.pause();
    let result = client.try_set_price(&asset, &100, &1000);
    assert!(result.is_err());
    let err = result.err().unwrap().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(OracleError::OraclePaused as u32)
    );
}

#[test]
fn test_pause_does_not_block_get_price() {
    let (_env, client, _admin) = setup_env();
    let asset = Address::generate(&_env);
    client.set_price(&asset, &100, &1000);
    client.pause();
    let price_data = client.get_price(&asset).unwrap();
    assert_eq!(price_data.price, 100);
}

#[test]
fn test_pause_non_admin_fails() {
    let (env, client, admin) = setup_env();
    client.pause();
    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);
}

#[test]
fn test_double_pause_fails() {
    let (_env, client, _admin) = setup_env();
    client.pause();
    let result = client.try_pause();
    assert!(result.is_err());
    let err = result.err().unwrap().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(OracleError::AlreadyPaused as u32)
    );
}

#[test]
fn test_unpause_success() {
    let (_env, client, _admin) = setup_env();
    let asset = Address::generate(&_env);
    client.pause();
    client.unpause();
    client.set_price(&asset, &100, &1000);
    let price_data = client.get_price(&asset).unwrap();
    assert_eq!(price_data.price, 100);
}

#[test]
fn test_unpause_non_admin_fails() {
    let (env, client, admin) = setup_env();
    client.pause();
    // Clear auths from pause()
    env.set_auths(&[]);
    client.unpause();
    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);
}

#[test]
#[should_panic(expected = "oracle is not paused")]
fn test_unpause_when_not_paused_fails() {
    let (_env, client, _admin) = setup_env();
    client.unpause();
}

#[test]
fn test_unpause_emits_event() {
    let (env, client, _admin) = setup_env();
    client.pause();
    client.unpause();
    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);
    let event_symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(event_symbol, Symbol::new(&env, "unpaused"));
}
