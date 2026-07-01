use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

pub(crate) fn setup_env() -> (Env, OracleContractClient<'static>, Address) {
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
    client.set_price(&admin, &asset, &100, &1000);

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
pub mod test_admin;
mod test_get_price_or_fail;
mod test_pause;
pub mod test_price;
pub mod test_redstone;
pub mod test_staleness;
