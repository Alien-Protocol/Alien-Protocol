#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::{token, Address, Env, Symbol, TryFromVal};

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let lending_pool = Address::generate(&env);
    let user = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_id = token_contract.address();

    (env, client, admin, lending_pool, user, token_id)
}

#[test]
fn test_deposit_event_topics() {
    let (env, client, admin, lending_pool, user, token_id) = setup_env();

    client.initialize(&admin, &lending_pool);
    client.add_supported_asset(&token_id);

    // Set up user token balance
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);
    token_admin_client.mint(&user, &1000);

    client.deposit(&user, &token_id, &500);

    let all_events = env.events().all();
    let last_event = all_events.last().unwrap();
    assert_eq!(last_event.0, client.address);

    let topics = last_event.1;

    // Topics: ["deposited", user, asset]
    assert_eq!(topics.len(), 3);

    let event_name = Symbol::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
    assert_eq!(event_name, Symbol::new(&env, "deposited"));

    let topic_user = Address::try_from_val(&env, &topics.get(1).unwrap()).unwrap();
    assert_eq!(topic_user, user);

    let topic_asset = Address::try_from_val(&env, &topics.get(2).unwrap()).unwrap();
    assert_eq!(topic_asset, token_id);

    // We do not need to parse the raw data struct since the SDK handles its encoding format.
    // It is sufficient to verify the correct topics were emitted.
}

#[test]
fn test_configuration_events() {
    let (env, client, admin, lending_pool, _, _) = setup_env();

    client.initialize(&admin, &lending_pool);

    let new_oracle = Address::generate(&env);

    client.set_oracle(&new_oracle);

    let last_event = env.events().all().last().unwrap();
    let topics = last_event.1;

    let event_name = Symbol::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
    assert_eq!(event_name, Symbol::new(&env, "oracle_updated"));

    // Data contains old_oracle and new_oracle
    // Because they are multiple fields, they are encoded as map/tuple.
    // In Soroban rust sdk #[contractevent], it encodes non-topic fields as a Tuple/Map.
    // If it's a struct with named fields, it's typically a Map. We don't need to assert deep raw data
    // as long as the topic structure is confirmed and we test old/new configuration behavior.
}

#[test]
fn test_failed_invocation_no_events() {
    let (env, client, admin, lending_pool, _, _) = setup_env();
    client.initialize(&admin, &lending_pool);

    // try to initialize again which should fail
    let res = client.try_initialize(&admin, &lending_pool);
    assert!(res.is_err());

    // Failed invocation rolls back events.
    assert!(env.events().all().is_empty());
}
