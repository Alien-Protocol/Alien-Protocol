#![cfg(test)]

use crate::tests::setup_env;
use crate::{OracleContract, OracleContractClient, OracleError};
use soroban_sdk::{
    testutils::{Address as _, Events, Ledger as _},
    Address, Env, Symbol, TryFromVal,
};

#[test]
fn test_set_staleness_threshold_success() {
    let (_env, client, _admin) = setup_env();
    client.set_staleness_threshold(&500_u64);

    let result = client.get_staleness_threshold();
    assert_eq!(result, Some(500));
}

#[test]
fn test_set_staleness_threshold_zero_fails() {
    let (_env, client, _admin) = setup_env();
    let result = client.try_set_staleness_threshold(&0_u64);
    assert!(result.is_err());
    let err = result.err().unwrap().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(OracleError::ThresholdZero as u32)
    );
}

#[test]
fn test_set_staleness_threshold_non_admin_fails() {
    let env = Env::default();
    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &300);

    let result = client.try_set_staleness_threshold(&500_u64);
    assert!(result.is_err());
}

#[test]
fn test_set_staleness_threshold_affects_freshness() {
    let (_env, client, _admin) = setup_env();
    let env = _env;
    let asset = Address::generate(&env);

    env.ledger().set_timestamp(1000);
    client.set_price(&_admin, &asset, &100, &100);

    // default threshold is 300 (from setup_env), so 900s diff is stale
    assert!(!client.is_price_fresh(&asset));

    // raise threshold to 1000, now it's fresh
    client.set_staleness_threshold(&1000_u64);
    assert!(client.is_price_fresh(&asset));
}

#[test]
fn test_set_staleness_threshold_emits_event() {
    let (env, client, _admin) = setup_env();
    client.set_staleness_threshold(&500_u64);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);
    let event_symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(
        event_symbol,
        Symbol::new(&env, "staleness_threshold_updated")
    );
}
