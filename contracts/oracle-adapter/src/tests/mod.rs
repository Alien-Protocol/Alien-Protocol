use super::*;
use soroban_sdk::testutils::{Address as _, Events, Ledger as _};
use soroban_sdk::{Address, Env, Symbol, TryFromVal};

pub(crate) fn setup_env() -> (Env, OracleContractClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    // Set a high initial ledger timestamp so set_price calls with typical test
    // timestamps (e.g. 1_000–100_000) are always <= ledger time.
    env.ledger().set_timestamp(1_000_000_000);

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
    client.set_price(&admin, &asset, &100, &1000);

    let price_data = client.get_price(&asset).unwrap();
    assert_eq!(price_data.price, 100);
    assert_eq!(price_data.timestamp, 1000);
}

#[test]
fn test_initialize_emits_event() {
    let env = Env::default();
    let contract_id = env.register(OracleContract, ());
    let client = OracleContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &300);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);
    let event_symbol = Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(event_symbol, Symbol::new(&env, "initialized"));
}

#[test]
fn test_initialize_twice_fails() {
    let (_env, client, admin) = setup_env();
    let result = client.try_initialize(&admin, &300);
    assert!(result.is_err());
    let err = result.err().unwrap().unwrap();
    assert_eq!(
        err,
        soroban_sdk::Error::from_contract_error(OracleError::AlreadyInitialized as u32)
    );
}
pub mod test_admin;
mod test_feeders;
mod test_get_price_or_fail;
mod test_pause;
pub mod test_price;
pub mod test_redstone;
pub mod test_staleness;
