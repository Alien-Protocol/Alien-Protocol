#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Events as _, MockAuth, MockAuthInvoke};
use soroban_sdk::{symbol_short, token, Address, Env, IntoVal, Map, Symbol, TryIntoVal, Val};

fn setup_initialized_vault<'a>(env: &'a Env) -> (Address, VaultContractClient<'a>, Address) {
    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    let oracle = Address::generate(env);

    client.initialize(&admin, &oracle);

    (contract_id, client, admin)
}

fn setup_vault_with_stored_admin<'a>(env: &'a Env) -> (Address, VaultContractClient<'a>, Address) {
    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(env, &contract_id);
    let admin = Address::generate(env);

    env.as_contract(&contract_id, || {
        storage::set_admin(env, &admin);
        storage::set_paused(env, false);
    });

    (contract_id, client, admin)
}

#[test]
fn test_vault_deposit_flow() {
    let env = Env::default();
    env.mock_all_auths();

    // Deploy and initialize contract
    let (contract_id, client, _admin) = setup_initialized_vault(&env);

    // Create address for user
    let user = Address::generate(&env);

    // Deploy standard token asset
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_contract_id = token_contract.address();
    let token_client = token::Client::new(&env, &token_contract_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_contract_id);

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

    // Pause vault and attempt user-facing operations
    client.pause();
    let err = client
        .try_deposit(&user, &token_contract_id, &100)
        .unwrap_err()
        .unwrap();
    assert_eq!(VaultError::try_from(err).unwrap(), VaultError::VaultPaused);

    let err = client
        .try_withdraw(&user, &token_contract_id, &100)
        .unwrap_err()
        .unwrap();
    assert_eq!(VaultError::try_from(err).unwrap(), VaultError::VaultPaused);

    // Unpause and deposit more
    client.set_paused(&false);
    client.deposit(&user, &token_contract_id, &200);

    assert_eq!(token_client.balance(&user), 300);
    assert_eq!(token_client.balance(&contract_id), 700);
    assert_eq!(client.get_position_balance(&user, &token_contract_id), 700);
}

#[test]
fn test_pause_requires_admin_auth_emits_event_and_rejects_double_pause() {
    let env = Env::default();

    let (contract_id, client, admin) = setup_vault_with_stored_admin(&env);
    let non_admin = Address::generate(&env);

    let res = client
        .mock_auths(&[MockAuth {
            address: &non_admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "pause",
                args: ().into_val(&env),
                sub_invokes: &[],
            },
        }])
        .try_pause();
    assert!(res.is_err(), "non-admin should not be able to pause");
    assert!(!env.as_contract(&contract_id, || storage::is_paused(&env)));

    client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &contract_id,
                fn_name: "pause",
                args: ().into_val(&env),
                sub_invokes: &[],
            },
        }])
        .pause();

    let events = env.events().all();
    assert_eq!(events.len(), 1);
    let (event_contract, topics, data) = events.get(0).unwrap();
    assert_eq!(event_contract, contract_id);
    assert_eq!(topics.len(), 1);

    let topic: Symbol = topics.get(0).unwrap().try_into_val(&env).unwrap();
    assert_eq!(topic, symbol_short!("paused"));

    let data: Map<Symbol, Val> = data.try_into_val(&env).unwrap();
    let by: Address = data
        .get(symbol_short!("by"))
        .unwrap()
        .try_into_val(&env)
        .unwrap();
    assert_eq!(by, admin);
    assert!(env.as_contract(&contract_id, || storage::is_paused(&env)));

    env.mock_all_auths();
    let err = client.try_pause().unwrap_err().unwrap();
    assert_eq!(
        VaultError::try_from(err).unwrap(),
        VaultError::AlreadyPaused,
        "double-pause should panic with a clear contract error"
    );
}
