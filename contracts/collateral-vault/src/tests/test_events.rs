#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::{token, Address, Env, Symbol, TryFromVal};

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
    Address, // admin
    Address, // lending_pool / oracle
    Address, // user
    Address, // token_id
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

// ── deposit event ────────────────────────────────────────────────────────────

#[test]
fn test_deposit_event_topics() {
    let (env, client, admin, lending_pool, user, token_id) = setup_env();

    client.initialize(&admin, &lending_pool);
    client.add_supported_asset(&token_id);

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
}

// ── configuration events ─────────────────────────────────────────────────────

#[test]
fn test_oracle_updated_event() {
    let (env, client, admin, lending_pool, _, _) = setup_env();

    client.initialize(&admin, &lending_pool);

    let new_oracle = Address::generate(&env);
    client.set_oracle(&new_oracle);

    let last_event = env.events().all().last().unwrap();
    let topics = last_event.1;

    let event_name = Symbol::try_from_val(&env, &topics.get(0).unwrap()).unwrap();
    assert_eq!(event_name, Symbol::new(&env, "oracle_updated"));
}

// ── failed invocations roll back events ──────────────────────────────────────

#[test]
fn test_double_initialize_fails_with_already_initialized() {
    let (env, client, admin, lending_pool, _, _) = setup_env();

    client.initialize(&admin, &lending_pool);

    // Capture the event count after a successful initialization.
    let events_after_first_init = env.events().all().len();
    assert!(events_after_first_init > 0, "initialize should emit at least one event");

    // Second call must return AlreadyInitialized.
    let err = client
        .try_initialize(&admin, &lending_pool)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, VaultError::AlreadyInitialized);

    // Failed invocations roll back their own events: the count must not have grown.
    assert_eq!(
        env.events().all().len(),
        events_after_first_init,
        "failed initialize must not emit additional events"
    );
}
