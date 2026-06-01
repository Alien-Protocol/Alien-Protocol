#![cfg(test)]

use soroban_sdk::{
    contract, contractimpl, contracttype, testutils::Address as _, Address, Env,
};

use crate::storage::{set_position, update_position_index};
use crate::types::Position;

#[contract]
struct MockLendingPool;

#[contracttype]
enum MockDataKey {
    Liquidatable,
}

#[contractimpl]
impl MockLendingPool {
    pub fn set_liquidatable(env: Env, liquidatable: bool) {
        env.storage()
            .instance()
            .set(&MockDataKey::Liquidatable, &liquidatable);
    }

    pub fn is_liquidatable(env: Env, _user: Address) -> bool {
        env.storage()
            .instance()
            .get(&MockDataKey::Liquidatable)
            .unwrap_or(false)
    }

    pub fn check_withdrawal_safe(
        _env: Env,
        _user: Address,
        _asset: Address,
        _amount: i128,
    ) -> bool {
        true
    }
}

fn setup_vault(env: &Env) -> (crate::VaultContractClient<'_>, Address) {
    let admin = Address::generate(env);
    let contract_id = env.register(crate::VaultContract, ());
    let client = crate::VaultContractClient::new(env, &contract_id);
    env.mock_all_auths();
    client.initialize(&admin, &Address::generate(env));
    (client, contract_id)
}

fn seed_position(env: &Env, contract_id: &Address, user: &Address, asset: &Address, amount: i128) {
    env.as_contract(contract_id, || {
        set_position(env, user, asset, &Position { amount });
        update_position_index(env, user, asset, amount);
    });
}

#[test]
fn test_authorize_liquidation_unauthorized_engine() {
    let env = Env::default();
    let liquidation_engine = Address::generate(&env);
    let unauthorized_engine = Address::generate(&env);
    let user = Address::generate(&env);

    let (client, _) = setup_vault(&env);
    client.set_liquidation_engine(&liquidation_engine);

    assert_eq!(
        client.authorize_liquidation(&unauthorized_engine, &user),
        false
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #2)")]
fn test_authorize_liquidation_missing_position() {
    let env = Env::default();
    let liquidation_engine = Address::generate(&env);
    let user = Address::generate(&env);
    let lending_pool = Address::generate(&env);

    let (client, _) = setup_vault(&env);
    client.set_liquidation_engine(&liquidation_engine);
    client.set_lending_pool(&lending_pool);

    client.authorize_liquidation(&liquidation_engine, &user);
}

#[test]
fn test_authorize_liquidation_liquidatable_position() {
    let env = Env::default();
    let liquidation_engine = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    let (client, contract_id) = setup_vault(&env);
    let lending_pool = env.register(MockLendingPool, ());
    let mock_pool = MockLendingPoolClient::new(&env, &lending_pool);
    mock_pool.set_liquidatable(&true);

    client.set_liquidation_engine(&liquidation_engine);
    client.set_lending_pool(&lending_pool);
    seed_position(&env, &contract_id, &user, &asset, 1_000);

    assert_eq!(client.authorize_liquidation(&liquidation_engine, &user), true);
}

#[test]
fn test_authorize_liquidation_non_liquidatable_position() {
    let env = Env::default();
    let liquidation_engine = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    let (client, contract_id) = setup_vault(&env);
    let lending_pool = env.register(MockLendingPool, ());
    let mock_pool = MockLendingPoolClient::new(&env, &lending_pool);
    mock_pool.set_liquidatable(&false);

    client.set_liquidation_engine(&liquidation_engine);
    client.set_lending_pool(&lending_pool);
    seed_position(&env, &contract_id, &user, &asset, 1_000);

    assert_eq!(client.authorize_liquidation(&liquidation_engine, &user), false);
}

#[test]
fn test_authorize_liquidation_no_liquidation_engine_set() {
    let env = Env::default();
    let liquidation_engine = Address::generate(&env);
    let user = Address::generate(&env);

    let (client, _) = setup_vault(&env);

    assert_eq!(client.authorize_liquidation(&liquidation_engine, &user), false);
}

#[test]
#[should_panic(expected = "Lending pool not set")]
fn test_authorize_liquidation_no_lending_pool_set() {
    let env = Env::default();
    let liquidation_engine = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);

    let (client, contract_id) = setup_vault(&env);
    client.set_liquidation_engine(&liquidation_engine);
    seed_position(&env, &contract_id, &user, &asset, 1_000);

    client.authorize_liquidation(&liquidation_engine, &user);
}
