#![cfg(test)]

extern crate std;

use crate::{OracleContract, OracleContractClient, OracleError, PriceData};
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, Env, Error};

fn setup_env() -> (Env, OracleContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    // Set timestamp >= price timestamps used in these tests (all ≤ 1_000).
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &300);

    (env, client, admin)
}

#[test]
fn test_get_price_or_fail_returns_fresh_price() {
    let (env, client, admin) = setup_env();
    let asset = Address::generate(&env);

    client.set_price(&admin, &asset, &1_000_i128, &1_000_u64);
    env.ledger().set_timestamp(1_100);

    let price_data: PriceData = client.get_price_or_fail(&asset);
    assert_eq!(price_data.price, 1_000);
    assert_eq!(price_data.timestamp, 1_000);
}

#[test]
fn test_get_price_or_fail_at_exact_threshold_succeeds() {
    let (env, client, admin) = setup_env();
    let asset = Address::generate(&env);

    client.set_price(&admin, &asset, &500_i128, &1_000_u64);
    // delta == threshold (300) is still fresh because the check is `delta <= threshold`
    env.ledger().set_timestamp(1_300);

    let price_data: PriceData = client.get_price_or_fail(&asset);
    assert_eq!(price_data.price, 500);
    assert_eq!(price_data.timestamp, 1_000);
}

#[test]
fn test_get_price_or_fail_unknown_asset_fails() {
    let (env, client, _admin) = setup_env();
    let unknown_asset = Address::generate(&env);

    let err = client
        .try_get_price_or_fail(&unknown_asset)
        .err()
        .unwrap()
        .unwrap();
    assert_eq!(
        err,
        Error::from_contract_error(OracleError::PriceNotFound as u32)
    );
}

#[test]
fn test_get_price_or_fail_stale_price_fails() {
    let (env, client, admin) = setup_env();
    let asset = Address::generate(&env);

    client.set_price(&admin, &asset, &1_000_i128, &1_000_u64);
    // delta = 2_000 - 1_000 = 1_000 > threshold (300) -> stale
    env.ledger().set_timestamp(2_000);

    let err = client.try_get_price_or_fail(&asset).err().unwrap().unwrap();
    assert_eq!(
        err,
        Error::from_contract_error(OracleError::StalePrice as u32)
    );
}
