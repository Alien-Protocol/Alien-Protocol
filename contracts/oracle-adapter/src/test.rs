#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Events, Ledger as _};
use soroban_sdk::{Address, Env, Symbol, TryFromVal};

fn setup_env() -> (Env, OracleContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &300);

    (env, client, admin)
}

#[test]
fn test_initialize_success() {
    let env = Env::default();
    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &300);

    env.mock_all_auths();
    let asset = Address::generate(&env);
    client.set_price(&asset, &100, &1000);

    let price_data = client.get_price(&asset).unwrap();
    assert_eq!(price_data.price, 100);
    assert_eq!(price_data.timestamp, 1000);
}

#[test]
#[should_panic(expected = "AlreadyInitialized")]
fn test_initialize_twice_fails() {
    let (_env, client, admin) = setup_env();
    client.initialize(&admin, &300);
}

#[test]
fn test_set_admin_success() {
    let (env, client, _admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    let asset = Address::generate(&env);
    client.set_price(&asset, &150, &2000);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, new_admin);
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
fn test_set_price_success() {
    let (env, client, admin) = setup_env();

    let asset = Address::generate(&env);
    client.set_price(&asset, &200, &3000);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);

    let price_opt = client.get_price(&asset);
    assert!(price_opt.is_some());
    let price_data = price_opt.unwrap();
    assert_eq!(price_data.price, 200);
    assert_eq!(price_data.timestamp, 3000);
}

#[test]
fn test_set_price_non_admin_fails() {
    let (env, client, admin) = setup_env();

    let asset = Address::generate(&env);
    client.set_price(&asset, &200, &3000);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);
}

#[test]
fn test_old_admin_cannot_call_set_price_after_transfer() {
    let (env, client, admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    let asset = Address::generate(&env);
    client.set_price(&asset, &300, &4000);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, new_admin);
    assert_ne!(*auth_addr, admin);
}

#[test]
fn test_get_price_is_public_and_unauthorized() {
    let (env, client, _admin) = setup_env();
    let asset = Address::generate(&env);

    client.set_price(&asset, &200, &3000);

    let price_opt = client.get_price(&asset);
    assert!(price_opt.is_some());
}

#[test]
fn test_set_price_zero_price_fails() {
    let (_env, client, _admin) = setup_env();
    let asset = Address::generate(&_env);

    let result = client.try_set_price(&asset, &0_i128, &100_000_u64);
    assert!(result.is_err());
}

#[test]
fn test_set_price_negative_price_fails() {
    let (_env, client, _admin) = setup_env();
    let asset = Address::generate(&_env);

    let result = client.try_set_price(&asset, &(-100_i128), &100_000_u64);
    assert!(result.is_err());
}

#[test]
fn test_set_price_zero_timestamp_fails() {
    let (_env, client, _admin) = setup_env();
    let asset = Address::generate(&_env);

    let result = client.try_set_price(&asset, &1000_i128, &0_u64);
    assert!(result.is_err());
}

#[test]
fn test_set_price_emits_event() {
    let (env, client, _admin) = setup_env();
    let asset = Address::generate(&env);

    client.set_price(&asset, &1000_i128, &100_000_u64);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);

    let event_symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(event_symbol, Symbol::new(&env, "price_updated"));
}

#[test]
fn test_set_price_different_assets() {
    let (_env, client, _admin) = setup_env();
    let asset1 = Address::generate(&_env);
    let asset2 = Address::generate(&_env);

    client.set_price(&asset1, &1000_i128, &100_000_u64);
    client.set_price(&asset2, &2000_i128, &200_000_u64);

    let price1 = client.get_price(&asset1).unwrap();
    assert_eq!(price1.price, 1000);

    let price2 = client.get_price(&asset2).unwrap();
    assert_eq!(price2.price, 2000);
}

#[test]
fn test_get_price_returns_correct_data() {
    let (_env, client, _admin) = setup_env();
    let env = _env;
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
    let (_env, client, _admin) = setup_env();
    let env = _env;
    let unknown_asset = Address::generate(&env);

    let result = client.get_price(&unknown_asset);
    assert!(result.is_none());
}

#[test]
fn test_get_price_does_not_mutate_state() {
    let (_env, client, _admin) = setup_env();
    let env = _env;
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
    let (_env, client, _admin) = setup_env();
    let env = _env;
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
    let (_env, client, _admin) = setup_env();
    let env = _env;
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

#[test]
fn test_is_price_fresh_unknown_asset() {
    let (_env, client, _admin) = setup_env();
    let env = _env;
    let unknown_asset = Address::generate(&env);

    assert!(!client.is_price_fresh(&unknown_asset));
}

#[test]
fn test_is_price_fresh_within_threshold() {
    let (_env, client, _admin) = setup_env();
    let env = _env;
    let asset = Address::generate(&env);

    env.ledger().set_timestamp(200);
    client.set_price(&asset, &100, &100);

    assert!(client.is_price_fresh(&asset));
}

#[test]
fn test_is_price_fresh_at_exact_threshold() {
    let (_env, client, _admin) = setup_env();
    let env = _env;
    let asset = Address::generate(&env);

    env.ledger().set_timestamp(400);
    client.set_price(&asset, &100, &100);

    assert!(client.is_price_fresh(&asset));
}

#[test]
fn test_is_price_fresh_stale() {
    let (_env, client, _admin) = setup_env();
    let env = _env;
    let asset = Address::generate(&env);

    env.ledger().set_timestamp(400);
    client.set_price(&asset, &100, &100);

    env.ledger().set_timestamp(401);

    assert!(!client.is_price_fresh(&asset));
}

#[test]
fn test_is_price_fresh_uninitialized_contract() {
    let env = Env::default();
    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let asset = Address::generate(&env);
    assert!(!client.is_price_fresh(&asset));
}

// ── Issue #511: get_admin ─────────────────────────────────────────────────────

#[test]
fn test_get_admin_returns_correct_address_after_init() {
    let (_env, client, admin) = setup_env();

    let result = client.get_admin();
    assert!(result.is_some());
    assert_eq!(result.unwrap(), admin);
}

#[test]
fn test_get_admin_returns_none_before_init() {
    let env = Env::default();
    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let result = client.get_admin();
    assert!(result.is_none());
}

#[test]
fn test_get_admin_does_not_mutate_state() {
    let (_env, client, admin) = setup_env();

    let result1 = client.get_admin();
    let result2 = client.get_admin();

    assert_eq!(result1, result2);
    assert_eq!(result1.unwrap(), admin);
}

#[test]
fn test_get_admin_returns_updated_admin_after_set_admin() {
    let (env, client, _old_admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    let result = client.get_admin();
    assert!(result.is_some());
    assert_eq!(result.unwrap(), new_admin);
}

#[test]
fn test_get_admin_requires_no_auth() {
    let (env, client, admin) = setup_env();

    env.set_auths(&[]);
    let result = client.get_admin();
    assert!(result.is_some());
    assert_eq!(result.unwrap(), admin);
}

// ── get_staleness_threshold ─────────────────────────────────────────────────────

#[test]
fn test_get_staleness_threshold_returns_correct_value_after_init() {
    let (_env, client, _admin) = setup_env();

    let result = client.get_staleness_threshold();
    assert_eq!(result, 300);
}

#[test]
fn test_get_staleness_threshold_panics_before_init() {
    let env = Env::default();
    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let result = client.try_get_staleness_threshold();
    assert!(result.is_err());
    let err = result.err().unwrap().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(OracleError::NotInitialized as u32)
    );
}

#[test]
fn test_get_staleness_threshold_does_not_mutate_state() {
    let (_env, client, _admin) = setup_env();

    let result1 = client.get_staleness_threshold();
    let result2 = client.get_staleness_threshold();

    assert_eq!(result1, result2);
    assert_eq!(result1, 300);
}

#[test]
fn test_get_staleness_threshold_requires_no_auth() {
    let (env, client, _admin) = setup_env();

    env.set_auths(&[]);
    let result = client.get_staleness_threshold();
    assert_eq!(result, 300);
}
