#![cfg(test)]

use super::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env};

#[test]
fn test_vault_deposit_flow() {
    let env = Env::default();
    env.mock_all_auths();

    // Deploy contract
    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    // Create address for admin, user, and oracle
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let oracle = Address::generate(&env);

    // Initialize vault
    client.initialize(&admin, &oracle);

    // Deploy standard token asset
    let token_admin = Address::generate(&env);
    let token_admin_client = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_contract_id = token_admin_client.address();
    let token_client = token::Client::new(&env, &token_contract_id);

    // Mint tokens to user
    token_admin_client.mint(&user, &1000);

    // Assert token balance before deposit
    assert_eq!(token_client.balance(&user), 1000);
    assert_eq!(token_client.balance(&contract_id), 0);

    // Try depositing unsupported asset -> should panic
    let res = client.try_deposit(&user, &token_contract_id, &500);
    assert!(res.is_err(), "should panic on unsupported asset");

    // Add asset to supported list
    client.add_supported_asset(&token_contract_id);
    assert!(client.is_supported_asset(&token_contract_id));

    // Try depositing <= 0 -> should panic
    let res = client.try_deposit(&user, &token_contract_id, &0);
    assert!(res.is_err(), "should panic on zero deposit");

    // Deposit 500
    client.deposit(&user, &token_contract_id, &500);

    // Check balances
    assert_eq!(token_client.balance(&user), 500);
    assert_eq!(token_client.balance(&contract_id), 500);

    // Check position balance in storage
    assert_eq!(client.get_position_balance(&user, &token_contract_id), 500);

    // Check position index
    let index = client.get_position_index();
    assert_eq!(index.len(), 1);
    assert_eq!(index.get(0).unwrap(), user);

    // Pause vault and attempt deposit
    client.set_paused(&true);
    let res = client.try_deposit(&user, &token_contract_id, &100);
    assert!(res.is_err(), "should panic when vault is paused");

    // Unpause and deposit more
    client.set_paused(&false);
    client.deposit(&user, &token_contract_id, &200);

    assert_eq!(token_client.balance(&user), 300);
    assert_eq!(token_client.balance(&contract_id), 700);
    assert_eq!(client.get_position_balance(&user, &token_contract_id), 700);
}
