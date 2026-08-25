#![cfg(test)]

extern crate std;

use crate::{OracleContract, OracleContractClient, OracleError, PriceData};
use soroban_sdk::testutils::{Address as _, Events, Ledger as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, Env, Map, Symbol, TryFromVal, Val, Vec};

fn setup_env_mock_all_auths() -> (Env, OracleContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    // Start ledger timestamp well above typical test timestamps (≤ 100_000)
    // so the future-timestamp guard in set_price never rejects test data.
    env.ledger().set_timestamp(1_000_000_000);

    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &300);

    (env, client, admin)
}

fn setup_env_no_mock_auths() -> (Env, OracleContractClient<'static>, Address) {
    let env = Env::default();
    // Start ledger timestamp well above typical test timestamps.
    env.ledger().set_timestamp(1_000_000_000);

    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &300);

    (env, client, admin)
}

#[test]
fn test_set_price_success() {
    let (env, client, admin) = setup_env_mock_all_auths();
    let asset = Address::generate(&env);

    client.set_price(&admin, &asset, &200_i128, &3000_u64);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);

    let stored = client.get_price(&asset).unwrap();
    assert_eq!(stored.price, 200);
    assert_eq!(stored.timestamp, 3000);
}

#[test]
fn test_set_price_zero_price_fails() {
    let (env, client, admin) = setup_env_mock_all_auths();
    let asset = Address::generate(&env);

    let result = client.try_set_price(&admin, &asset, &0_i128, &100_000_u64);
    assert!(result.is_err());
}

#[test]
fn test_set_price_negative_price_fails() {
    let (env, client, admin) = setup_env_mock_all_auths();
    let asset = Address::generate(&env);

    let result = client.try_set_price(&admin, &asset, &(-100_i128), &100_000_u64);
    assert!(result.is_err());
}

#[test]
fn test_set_price_non_admin_fails() {
    // IMPORTANT: no mock_all_auths here, otherwise auth always succeeds.
    let (env, client, _admin) = setup_env_no_mock_auths();

    let non_admin = Address::generate(&env);
    let asset = Address::generate(&env);

    // Authorized invocation tree for *non_admin* (but contract requires stored admin)
    let args = Vec::<Val>::new(&env);
    let invoke = MockAuthInvoke {
        contract: &client.address,
        fn_name: "set_price",
        args,
        sub_invokes: &[],
    };

    env.mock_auths(&[MockAuth {
        address: &non_admin,
        invoke: &invoke,
    }]);

    let result = client.try_set_price(&non_admin, &asset, &123_i128, &999_u64);
    assert!(result.is_err());
}

#[test]
fn test_set_price_emits_event() {
    let (env, client, admin) = setup_env_mock_all_auths();
    let asset = Address::generate(&env);

    let price = 1000_i128;
    let timestamp = 100_000_u64;

    client.set_price(&admin, &asset, &price, &timestamp);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);

    // topic symbol
    let event_symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(event_symbol, Symbol::new(&env, "price_updated"));

    // data is a map: { asset: Address, price: i128, timestamp: u64 }
    let data: Map<Symbol, Val> = Map::try_from_val(&env, &last_event.2).unwrap();

    let asset_val = data.get(Symbol::new(&env, "asset")).unwrap();
    let price_val = data.get(Symbol::new(&env, "price")).unwrap();
    let ts_val = data.get(Symbol::new(&env, "timestamp")).unwrap();

    let emitted_asset = Address::try_from_val(&env, &asset_val).unwrap();
    let emitted_price = i128::try_from_val(&env, &price_val).unwrap();
    let emitted_ts = u64::try_from_val(&env, &ts_val).unwrap();

    assert_eq!(emitted_asset, asset);
    assert_eq!(emitted_price, price);
    assert_eq!(emitted_ts, timestamp);
}

#[test]
fn test_get_price_returns_correct_data() {
    let (env, client, admin) = setup_env_mock_all_auths();
    let asset = Address::generate(&env);

    let price = 1_000_000_i128;
    // Keep timestamp <= ledger time (1_000_000_000 set in setup_env_mock_all_auths).
    let timestamp = 999_999_999_u64;

    client.set_price(&admin, &asset, &price, &timestamp);

    let result = client.get_price(&asset);
    assert!(result.is_some());

    let price_data: PriceData = result.unwrap();
    assert_eq!(price_data.price, price);
    assert_eq!(price_data.timestamp, timestamp);
}

#[test]
fn test_get_price_unknown_asset_returns_none() {
    let (env, client, _admin) = setup_env_mock_all_auths();
    let unknown_asset = Address::generate(&env);

    let result = client.get_price(&unknown_asset);
    assert!(result.is_none());
}

#[test]
fn test_set_price_overwrites_existing() {
    let (env, client, admin) = setup_env_mock_all_auths();
    let asset = Address::generate(&env);

    client.set_price(&admin, &asset, &111_i128, &100_u64);
    let first = client.get_price(&asset).unwrap();
    assert_eq!(first.price, 111);
    assert_eq!(first.timestamp, 100);

    client.set_price(&admin, &asset, &222_i128, &200_u64);
    let second = client.get_price(&asset).unwrap();
    assert_eq!(second.price, 222);
    assert_eq!(second.timestamp, 200);
}

// ── Issue #647: stale and future timestamp guards ────────────────────────────

/// set_price must reject a timestamp older than the last stored price timestamp.
/// Acceptance criterion: "Older timestamp for the same asset fails".
#[test]
fn test_set_price_older_timestamp_rejected() {
    let (env, client, admin) = setup_env_mock_all_auths();
    let asset = Address::generate(&env);

    // First write at timestamp 5_000.
    client.set_price(&admin, &asset, &100_i128, &5_000_u64);

    // Second write with an older timestamp must fail with InvalidTimestamp.
    let result = client.try_set_price(&admin, &asset, &200_i128, &4_999_u64);
    assert!(result.is_err());
    let err = result.err().unwrap().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(OracleError::InvalidTimestamp as u32)
    );
}

/// set_price must also reject a timestamp equal to the last stored price timestamp
/// (requires *strictly* newer, not merely non-decreasing).
#[test]
fn test_set_price_equal_timestamp_rejected() {
    let (env, client, admin) = setup_env_mock_all_auths();
    let asset = Address::generate(&env);

    client.set_price(&admin, &asset, &100_i128, &5_000_u64);

    let result = client.try_set_price(&admin, &asset, &200_i128, &5_000_u64);
    assert!(result.is_err());
    let err = result.err().unwrap().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(OracleError::InvalidTimestamp as u32)
    );
}

/// set_price must reject a timestamp that exceeds the current ledger time.
/// Acceptance criterion: "Timestamp after ledger time fails".
#[test]
fn test_set_price_future_timestamp_rejected() {
    let (env, client, admin) = setup_env_mock_all_auths();
    let asset = Address::generate(&env);

    // Pin the ledger to a known time, then try to submit a price timestamped
    // one second in the future.
    let ledger_now: u64 = 50_000;
    env.ledger().set_timestamp(ledger_now);

    let result = client.try_set_price(&admin, &asset, &300_i128, &(ledger_now + 1));
    assert!(result.is_err());
    let err = result.err().unwrap().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(OracleError::InvalidTimestamp as u32)
    );
}

/// A price timestamped exactly at ledger time is valid (boundary: timestamp == ledger).
#[test]
fn test_set_price_at_ledger_timestamp_succeeds() {
    let (env, client, admin) = setup_env_mock_all_auths();
    let asset = Address::generate(&env);

    let ledger_now: u64 = 50_000;
    env.ledger().set_timestamp(ledger_now);

    // timestamp == ledger time must be accepted.
    client.set_price(&admin, &asset, &300_i128, &ledger_now);
    let stored = client.get_price(&asset).unwrap();
    assert_eq!(stored.price, 300);
    assert_eq!(stored.timestamp, ledger_now);
}

/// The very first set_price for a new asset succeeds even when no prior price
/// exists (stale-check branch is not taken).
/// Acceptance criterion: "First valid set_price still succeeds".
#[test]
fn test_set_price_first_write_succeeds() {
    let (env, client, admin) = setup_env_mock_all_auths();
    let asset = Address::generate(&env);

    // No prior price — should succeed without hitting the stale guard.
    client.set_price(&admin, &asset, &500_i128, &1_000_u64);
    let stored = client.get_price(&asset).unwrap();
    assert_eq!(stored.price, 500);
    assert_eq!(stored.timestamp, 1_000);
}
