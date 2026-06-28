#![cfg(test)]

extern crate std;

use crate::{OracleContract, OracleContractClient, PriceData};
use soroban_sdk::testutils::{Address as _, Events, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, Env, Map, Symbol, TryFromVal, Val, Vec};

fn setup_env_mock_all_auths() -> (Env, OracleContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &300);

    (env, client, admin)
}

fn setup_env_no_mock_auths() -> (Env, OracleContractClient<'static>, Address) {
    let env = Env::default();

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
    let timestamp = 1_234_567_890_u64;

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
