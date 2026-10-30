#![cfg(test)]

extern crate std;

use crate::tests::setup_env;
use crate::{OracleContractClient, OracleError};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Bytes, Env, Symbol, Vec,
};

fn hex_to_bytes(env: &Env, hex: &str) -> Bytes {
    let mut res = std::vec::Vec::new();
    for i in (0..hex.len()).step_by(2) {
        let b = u8::from_str_radix(&hex[i..i + 2], 16).unwrap();
        res.push(b);
    }
    Bytes::from_slice(env, &res)
}

fn set_ledger_time(env: &Env, timestamp_seconds: u64) {
    let mut ledger_info = env.ledger().get();
    ledger_info.timestamp = timestamp_seconds;
    env.ledger().set(ledger_info);
}

const PRIMARY_SIGNERS: [&str; 5] = [
    "8bb8f32df04c8b654987daaed53d6b6091e3b774",
    "deb22f54738d54976c4c0fe5ce6d408e40d88499",
    "51ce04be4b3e32572c4ec9135221d0691ba7d202",
    "dd682daec5a90dd295d14da4b0bec9281017b5be",
    "9c5ae89c4af6aa32ce58588dbaf90d18a855b6de",
];

// ETH_BTC_PRIMARY_3sig.hex (timestamp: 1744829560000 ms = 1744829560 s, BTC: 8396083019375, ETH: 156537608660)
const PAYLOAD_3SIG_HEX: &str = "4254430000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007a2dd8bbe6f01963ff234c000000020000001cdf5adae1ece03869a5027f081a512501a5ab63300872a32c91bd82ef78ebeb62214b730ef56dc13e60951a983e2c7bddeb00d8c240b849ba391df5d05caf4111b4254430000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007a2dd8bbe6f01963ff234c00000002000000152637c8e49e2ee5b37d19c8dd047c9ccff51f79a4fb3ef04998ad72d9ebf3a655d51fda162f6a6aa10a202d106e6a805a74eb53d99f791fd7e92460dbf455d251b4254430000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007a2dd7c7c3401963ff234c000000020000001ebfce932f7df388b87df530ea12e04928f3455f31042cf8ac45bd1ba6a4fe59f6384838d7b2ae18d254214a91a5d041220576ab5ccbe33580bc1e829e51780d41b455448000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000024725cb7e401963ff234c000000020000001bb58f27a77b2d9da01a51c1a4a5e4243db161b41a4c4dc4a2c59045da9860a16762cea54fe5918c45c8f8eb8b22e9b8ee558cb175db5b790880bf437d1a3de1b1c455448000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000024725e59d401963ff234c00000002000000124e4d492822578be6709dfdb6b19cbb2fbe031dccc0c903d99ca5035556d8c917fd7ed6b1e32b351fd261d355bc8fe1cda7203a66d037e2953e11ddcb6a768931b4554480000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000247282d70001963ff234c0000000200000010468da6687feed5f49745b59dda081e64c5ab3b73d36b734b29969dde94ab2ce7eeccd1660bebf14bf3272d5c5e937c30101ca3907a2647a9b2ee8c98abe7ea81b0006000000000002ed57011e0000";

fn setup_redstone(env: &Env, client: &OracleContractClient<'static>, admin: &Address) {
    let mut signers = Vec::new(env);
    for s in PRIMARY_SIGNERS.iter() {
        signers.push_back(hex_to_bytes(env, s));
    }
    client.set_redstone_config(admin, &signers, &3);
}

#[test]
fn test_set_asset_feed_requires_admin() {
    let (env, client, admin) = setup_env();
    let attacker = Address::generate(&env);
    let asset = Address::generate(&env);
    let btc = Symbol::new(&env, "BTC");

    // Non-admin call fails with Unauthorized
    let res = client.try_set_asset_feed(&attacker, &asset, &btc);
    assert_eq!(res.err().unwrap().unwrap(), OracleError::Unauthorized);

    // Admin call succeeds
    let ok_res = client.try_set_asset_feed(&admin, &asset, &btc);
    assert!(ok_res.is_ok());

    // Reading asset feed returns the mapped symbol
    assert_eq!(client.get_asset_feed(&asset), Some(btc));
}

#[test]
fn test_get_price_uses_mapped_feed() {
    let (env, client, admin) = setup_env();
    setup_redstone(&env, &client, &admin);

    let asset = Address::generate(&env);
    let btc = Symbol::new(&env, "BTC");
    client.set_asset_feed(&admin, &asset, &btc);

    let mut feeds = Vec::new(&env);
    feeds.push_back(btc);

    set_ledger_time(&env, 1744829560);
    let payload = hex_to_bytes(&env, PAYLOAD_3SIG_HEX);
    client.write_prices(&admin, &feeds, &payload);

    let price_data = client.get_price(&asset).expect("mapped price should exist");
    assert_eq!(price_data.price, 8396083019375);
    assert_eq!(price_data.timestamp, 1744829560000);
}

#[test]
fn test_get_price_or_fail_stale_mapped_feed() {
    let (env, client, admin) = setup_env();
    setup_redstone(&env, &client, &admin);

    let asset = Address::generate(&env);
    let btc = Symbol::new(&env, "BTC");
    client.set_asset_feed(&admin, &asset, &btc);

    let mut feeds = Vec::new(&env);
    feeds.push_back(btc);

    // Write at time 1744829560
    set_ledger_time(&env, 1744829560);
    let payload = hex_to_bytes(&env, PAYLOAD_3SIG_HEX);
    client.write_prices(&admin, &feeds, &payload);

    // Set ledger past the 300s threshold
    set_ledger_time(&env, 1744829560 + 500);

    let res = client.try_get_price_or_fail(&asset);
    assert_eq!(
        res.err().unwrap().unwrap(),
        soroban_sdk::Error::from_contract_error(OracleError::StalePrice as u32)
    );
}

#[test]
fn test_write_prices_updates_mapped_asset_price() {
    let (env, client, admin) = setup_env();
    setup_redstone(&env, &client, &admin);

    let eth_asset = Address::generate(&env);
    let eth = Symbol::new(&env, "ETH");
    client.set_asset_feed(&admin, &eth_asset, &eth);

    let mut feeds = Vec::new(&env);
    feeds.push_back(eth);

    set_ledger_time(&env, 1744829560);
    let payload = hex_to_bytes(&env, PAYLOAD_3SIG_HEX);
    client.write_prices(&admin, &feeds, &payload);

    let price_data = client
        .get_price(&eth_asset)
        .expect("ETH asset price mapped");
    assert_eq!(price_data.price, 156537608660);
    assert_eq!(price_data.timestamp, 1744829560000);
}

#[test]
fn test_unmapped_asset_keeps_address_price_path() {
    let (env, client, admin) = setup_env();

    let asset = Address::generate(&env);
    assert_eq!(client.get_asset_feed(&asset), None);

    // Unmapped asset uses address-keyed set_price / get_price
    client.set_price(&admin, &asset, &100_000_000, &1_000_000_000);
    let price_data = client.get_price(&asset).expect("address-keyed price");
    assert_eq!(price_data.price, 100_000_000);
    assert_eq!(price_data.timestamp, 1_000_000_000);

    let direct_or_fail = client.get_price_or_fail(&asset);
    assert_eq!(direct_or_fail.price, 100_000_000);
}
