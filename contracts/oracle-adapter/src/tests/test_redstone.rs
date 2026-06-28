#![cfg(test)]

extern crate std;

use crate::OracleContractClient;
use crate::tests::setup_env;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Bytes, Env, Symbol, Vec,
};

fn hex_to_bytes(env: &Env, hex: &str) -> Bytes {
    let mut res = std::vec::Vec::new();
    for i in (0..hex.len()).step_by(2) {
        let b = u8::from_str_radix(&hex[i..i+2], 16).unwrap();
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

// ETH_BTC_PRIMARY_3sig.hex (timestamp: 1744829560000 ms, BTC: 8396083019375, ETH: 156537608660)
const PAYLOAD_3SIG_HEX: &str = "4254430000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007a2dd8bbe6f01963ff234c000000020000001cdf5adae1ece03869a5027f081a512501a5ab63300872a32c91bd82ef78ebeb62214b730ef56dc13e60951a983e2c7bddeb00d8c240b849ba391df5d05caf4111b4254430000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007a2dd8bbe6f01963ff234c00000002000000152637c8e49e2ee5b37d19c8dd047c9ccff51f79a4fb3ef04998ad72d9ebf3a655d51fda162f6a6aa10a202d106e6a805a74eb53d99f791fd7e92460dbf455d251b4254430000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007a2dd7c7c3401963ff234c000000020000001ebfce932f7df388b87df530ea12e04928f3455f31042cf8ac45bd1ba6a4fe59f6384838d7b2ae18d254214a91a5d041220576ab5ccbe33580bc1e829e51780d41b455448000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000024725cb7e401963ff234c000000020000001bb58f27a77b2d9da01a51c1a4a5e4243db161b41a4c4dc4a2c59045da9860a16762cea54fe5918c45c8f8eb8b22e9b8ee558cb175db5b790880bf437d1a3de1b1c455448000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000024725e59d401963ff234c00000002000000124e4d492822578be6709dfdb6b19cbb2fbe031dccc0c903d99ca5035556d8c917fd7ed6b1e32b351fd261d355bc8fe1cda7203a66d037e2953e11ddcb6a768931b4554480000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000247282d70001963ff234c0000000200000010468da6687feed5f49745b59dda081e64c5ab3b73d36b734b29969dde94ab2ce7eeccd1660bebf14bf3272d5c5e937c30101ca3907a2647a9b2ee8c98abe7ea81b0006000000000002ed57011e0000";

// ETH_BTC_PRIMARY_3sig_newer.hex (timestamp: 1744829650000 ms, BTC: 8396977516955, ETH: 156277937205)
const PAYLOAD_3SIG_NEWER_HEX: &str = "4254430000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007a312dcb19b01963ff394500000002000000169e535fd1aea6339619ffc938218f570d18666405e0e80ee9561534ce0b91f253c086d40e7b14e68019f15ada697a862d49be70e3efbca39cb246b128984b8f51c4254430000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007a312dcb19b01963ff3945000000020000001055ef5733b9a07009c2e7f7c19a54cc6ce01224b8cbea623b0f7882f1e91b743558093874380c48fba14f09a83971f5ace880932c1d11b7c6557168214d31abb1c4254430000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000007a312dcb19b01963ff3945000000020000001319861075bb127635110989abe513f9921ad18fabbf05aeff600f9cad83d6f051dd3b382551e19aed92fe9593c7f981acfa0a3b0df0bc564433d281ec27907211c45544800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002462e4143501963ff3945000000020000001078bba7675c4376b34f9254fe0ba7de823ad51830cc9d32ffaee6b8ea942ef4b42300b14c40e78e72d5b4cf880f969b1defa00cfb543ce284791748b779a84791b45544800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002462e4143501963ff3945000000020000001519b4b81db0092aa1faad2c5df0aad42082923fb36e425c0d27af76cb344052044afce5f946243e628ca7f3fd990815b71dcc339de6c7a8467dd6a113485cef11c45544800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002462e4143501963ff3945000000020000001d3e6d08ba9e903e86fb3699c09312d4ca85ce3277bb6dbbc672dc7f091d1f61e6eb44d95d7333cdd28df47de3cdb14940bc823b18fc34023b909e767055ca6451c0006000000000002ed57011e0000";

fn setup_redstone_config(env: &Env, client: &OracleContractClient<'static>, admin: &Address) {
    let mut signers = Vec::new(env);
    for s in PRIMARY_SIGNERS.iter() {
        signers.push_back(hex_to_bytes(env, s));
    }
    client.set_redstone_config(admin, &signers, &3);
}

#[test]
fn test_set_get_redstone_config() {
    let (env, client, admin) = setup_env();

    // Not initialized initially
    let get_res = client.try_get_redstone_config();
    assert!(get_res.is_err());

    setup_redstone_config(&env, &client, &admin);

    let (signers, threshold) = client.get_redstone_config();
    assert_eq!(threshold, 3);
    assert_eq!(signers.len(), 5);
}

#[test]
fn test_pull_model_success() {
    let (env, client, admin) = setup_env();
    setup_redstone_config(&env, &client, &admin);

    // Set ledger timestamp to match payload timestamp (1744829560 seconds)
    set_ledger_time(&env, 1744829560);

    let payload = hex_to_bytes(&env, PAYLOAD_3SIG_HEX);
    let feed_ids = Vec::from_array(&env, [Symbol::new(&env, "BTC"), Symbol::new(&env, "ETH")]);

    let (timestamp, prices) = client.get_prices(&feed_ids, &payload);

    assert_eq!(timestamp, 1744829560000);
    assert_eq!(prices.get(0).unwrap(), 8396083019375);
    assert_eq!(prices.get(1).unwrap(), 156537608660);
}

#[test]
fn test_pull_model_unknown_feed() {
    let (env, client, admin) = setup_env();
    setup_redstone_config(&env, &client, &admin);
    set_ledger_time(&env, 1744829560);

    let payload = hex_to_bytes(&env, PAYLOAD_3SIG_HEX);
    // DOGE is not supported/recognized
    let feed_ids = Vec::from_array(&env, [Symbol::new(&env, "DOGE")]);

    let result = client.try_get_prices(&feed_ids, &payload);
    assert!(result.is_err());
}

#[test]
fn test_pull_model_invalid_payload() {
    let (env, client, admin) = setup_env();
    setup_redstone_config(&env, &client, &admin);
    set_ledger_time(&env, 1744829560);

    // Tampered payload
    let mut payload_bytes = hex_to_bytes(&env, PAYLOAD_3SIG_HEX);
    payload_bytes.set(0, 0); // corrupt the first byte

    let feed_ids = Vec::from_array(&env, [Symbol::new(&env, "BTC")]);

    let result = client.try_get_prices(&feed_ids, &payload_bytes);
    assert!(result.is_err());
}

#[test]
fn test_push_model_success() {
    let (env, client, admin) = setup_env();
    setup_redstone_config(&env, &client, &admin);

    set_ledger_time(&env, 1744829560);

    let payload = hex_to_bytes(&env, PAYLOAD_3SIG_HEX);
    let feed_ids = Vec::from_array(&env, [Symbol::new(&env, "BTC"), Symbol::new(&env, "ETH")]);

    // Feed is not written yet
    let read_err = client.try_read_prices(&feed_ids);
    assert!(read_err.is_err());

    // Write prices
    client.write_prices(&admin, &feed_ids, &payload);

    // Read successfully
    let stored = client.read_prices(&feed_ids);
    assert_eq!(stored.len(), 2);
    
    let btc_data = stored.get(0).unwrap();
    assert_eq!(btc_data.price, 8396083019375);
    assert_eq!(btc_data.timestamp, 1744829560000);
    assert_eq!(btc_data.write_timestamp, 1744829560);

    let eth_data = stored.get(1).unwrap();
    assert_eq!(eth_data.price, 156537608660);
    assert_eq!(eth_data.timestamp, 1744829560000);
    assert_eq!(eth_data.write_timestamp, 1744829560);
}

#[test]
fn test_push_model_silent_reject_stale() {
    let (env, client, admin) = setup_env();
    setup_redstone_config(&env, &client, &admin);

    // 1. Write the newer price first (timestamp: 1744829650)
    set_ledger_time(&env, 1744829650);
    let payload_newer = hex_to_bytes(&env, PAYLOAD_3SIG_NEWER_HEX);
    let feed_ids = Vec::from_array(&env, [Symbol::new(&env, "BTC"), Symbol::new(&env, "ETH")]);

    client.write_prices(&admin, &feed_ids, &payload_newer);

    let stored_newer = client.read_prices(&feed_ids);
    let btc_newer = stored_newer.get(0).unwrap();
    assert_eq!(btc_newer.price, 8396977516955); // newer price

    // 2. Try to write the older price (timestamp: 1744829560)
    set_ledger_time(&env, 1744829660); // current block time is even newer
    let payload_older = hex_to_bytes(&env, PAYLOAD_3SIG_HEX);
    client.write_prices(&admin, &feed_ids, &payload_older);

    // 3. Read again - should NOT be overwritten (remains newer price)
    let stored_after = client.read_prices(&feed_ids);
    let btc_after = stored_after.get(0).unwrap();
    assert_eq!(btc_after.price, 8396977516955);
}

#[test]
fn test_push_model_unauthorized() {
    let (env, client, admin) = setup_env();
    setup_redstone_config(&env, &client, &admin);
    set_ledger_time(&env, 1744829560);

    let non_admin = Address::generate(&env);
    let payload = hex_to_bytes(&env, PAYLOAD_3SIG_HEX);
    let feed_ids = Vec::from_array(&env, [Symbol::new(&env, "BTC")]);

    // Call from non-admin should fail/unauthorized
    let result = client.try_write_prices(&non_admin, &feed_ids, &payload);
    assert!(result.is_err());
}
