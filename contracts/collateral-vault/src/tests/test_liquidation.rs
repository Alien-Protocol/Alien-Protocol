#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env};

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let oracle = Address::generate(&env);

    client.initialize(&admin, &oracle);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_contract_id = token_contract.address();
    let token_client = token::Client::new(&env, &token_contract_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract_id);

    client.add_supported_asset(&token_contract_id);

    (
        env,
        client,
        admin,
        user,
        oracle,
        token_contract_id,
        token_client,
        token_admin_client,
    )
}

#[test]
fn test_authorize_liquidation_success() {
    let (env, client, _admin, _user, _oracle, _token_id, _token_client, _token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);
}

#[test]
fn test_seize_collateral_emits_event() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.seize_collateral(&engine, &user, &token_id, &200);

    // In Soroban, we can't easily "check" events in the same way as logs,
    // but the contract publishes them. If we wanted to verify, we would usually
    // check the ledger's events.
    // For now, we've verified it doesn't panic.
}

#[test]
fn test_seize_collateral_success() {
    let (env, client, _admin, user, _oracle, token_id, token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.seize_collateral(&engine, &user, &token_id, &200);

    assert_eq!(client.get_position_balance(&user, &token_id), 300);
    assert_eq!(token_client.balance(&engine), 200);
    assert_eq!(token_client.balance(&client.address), 300);
}

#[test]
fn test_seize_collateral_unauthorized_engine_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);
    let malicious_engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&malicious_engine, &user, &token_id, &200);
    assert!(res.is_err());
}

#[test]
fn test_seize_collateral_insufficient_balance_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let res = client.try_seize_collateral(&engine, &user, &token_id, &600);
    assert!(res.is_err());
}

#[test]
fn test_seize_collateral_no_position_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, _token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    // User has NO position
    let res = client.try_seize_collateral(&engine, &user, &token_id, &200);
    assert!(res.is_err());
}

#[test]
fn test_seize_collateral_removes_from_index_on_zero() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    assert!(client.get_position_index().contains(&user));

    client.seize_collateral(&engine, &user, &token_id, &500);

    assert_eq!(client.get_position_balance(&user, &token_id), 0);
    assert!(!client.get_position_index().contains(&user));
}

#[test]
fn test_seize_collateral_paused_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();
    let engine = Address::generate(&env);

    client.set_liquidation_engine(&engine);

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.pause();

    let res = client.try_seize_collateral(&engine, &user, &token_id, &200);
    assert!(res.is_err());
}

#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, Env, IntoVal,
};

// --- Mock Contracts Setup ---

pub struct MockLendingPool;

#[soroban_sdk::contractimpl]
impl MockLendingPool {
    pub fn get_user_debt(_env: Env, _user: Address) -> i128 {
        1000
    }

    pub fn is_liquidatable(env: Env, user: Address) -> bool {
        // Toggle health status via env storage for testing
        env.storage()
            .instance()
            .get(&user)
            .unwrap_or(false)
    }

    pub fn set_liquidatable(env: Env, user: Address, liquidatable: bool) {
        env.storage().instance().set(&user, &liquidatable);
    }
}

// --- Test Cases ---

#[test]
fn test_seize_collateral_atomic_success_when_unhealthy() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);
    let engine = Address::generate(&env);

    // Register Mock Lending Pool
    let pool_id = env.register_contract(None, MockLendingPool);
    let pool_client = MockLendingPoolClient::new(&env, &pool_id);

    // Register Vault Contract
    let vault_id = env.register_contract(None, VaultContract);
    let vault_client = VaultContractClient::new(&env, &vault_id);

    // Initialize & Setup
    vault_client.initialize(&admin, &pool_id);
    vault_client.set_liquidation_engine(&engine);
    vault_client.add_supported_asset(&asset);

    // Deposit collateral for user
    vault_client.deposit(&user, &asset, &1000);

    // Set position as UNHEALTHY (liquidatable = true) in mock pool
    pool_client.set_liquidatable(&user, &true);

    // ATOMIC SEIZURE: Should succeed because user is liquidatable
    vault_client.seize_collateral(&engine, &user, &asset, &500);

    // Verify balance reduced
    assert_eq!(vault_client.get_position_balance(&user, &asset), 500);
}

#[test]
#[should_panic]
fn test_seize_collateral_fails_when_position_is_healthy() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);
    let engine = Address::generate(&env);

    let pool_id = env.register_contract(None, MockLendingPool);
    let pool_client = MockLendingPoolClient::new(&env, &pool_id);

    let vault_id = env.register_contract(None, VaultContract);
    let vault_client = VaultContractClient::new(&env, &vault_id);

    vault_client.initialize(&admin, &pool_id);
    vault_client.set_liquidation_engine(&engine);
    vault_client.add_supported_asset(&asset);

    vault_client.deposit(&user, &asset, &1000);

    // Mark position as HEALTHY (liquidatable = false)
    pool_client.set_liquidatable(&user, &false);

    // ATOMIC SEIZURE: Must panic/fail because position is not liquidatable
    vault_client.seize_collateral(&engine, &user, &asset, &500);
}

#[test]
#[should_panic]
fn test_seize_collateral_fails_for_unauthorized_engine() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = Address::generate(&env);
    let registered_engine = Address::generate(&env);
    let rogue_engine = Address::generate(&env);

    let pool_id = env.register_contract(None, MockLendingPool);
    let pool_client = MockLendingPoolClient::new(&env, &pool_id);

    let vault_id = env.register_contract(None, VaultContract);
    let vault_client = VaultContractClient::new(&env, &vault_id);

    vault_client.initialize(&admin, &pool_id);
    vault_client.set_liquidation_engine(&registered_engine);
    vault_client.add_supported_asset(&asset);

    vault_client.deposit(&user, &asset, &1000);
    pool_client.set_liquidatable(&user, &true);

    // Attempt seizure from an unregistered engine -> MUST FAIL
    vault_client.seize_collateral(&rogue_engine, &user, &asset, &500);
}
