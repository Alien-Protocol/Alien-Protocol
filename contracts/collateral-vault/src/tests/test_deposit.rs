#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::{token, Address, Env, Symbol};

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
    let lending_pool = Address::generate(&env);
    let liquidation_engine = Address::generate(&env);

    client.initialize(&admin, &lending_pool, &oracle, &liquidation_engine);

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
fn test_deposit_success() {
    let (_env, client, _admin, user, _oracle, token_id, token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    assert_eq!(token_client.balance(&user), 500);
    assert_eq!(token_client.balance(&client.address), 500);
    assert_eq!(client.get_position_balance(&user, &token_id), 500);
}

#[test]
fn test_deposit_increases_existing_balance() {
    let (_env, client, _admin, user, _oracle, token_id, token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &300);
    client.deposit(&user, &token_id, &400);

    assert_eq!(token_client.balance(&user), 300);
    assert_eq!(token_client.balance(&client.address), 700);
    assert_eq!(client.get_position_balance(&user, &token_id), 700);
}

#[test]
fn test_deposit_zero_amount_fails() {
    let (_env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    let res = client.try_deposit(&user, &token_id, &0);
    assert_eq!(
        res,
        Err(Ok(soroban_sdk::Error::from_contract_error(
            VaultError::InvalidAmount as u32
        )))
    );
}

#[test]
fn test_deposit_negative_amount_fails() {
    let (_env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    let res = client.try_deposit(&user, &token_id, &-100);
    assert_eq!(
        res,
        Err(Ok(soroban_sdk::Error::from_contract_error(
            VaultError::InvalidAmount as u32
        )))
    );
}

#[test]
fn test_deposit_uses_checked_credit_and_updates_index() {
    let (_env, client, _admin, user, _oracle, token_id, token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    assert_eq!(token_client.balance(&user), 500);
    assert_eq!(token_client.balance(&client.address), 500);
    assert_eq!(client.get_position_balance(&user, &token_id), 500);

    let index = client.get_position_index();
    assert_eq!(index.len(), 1);
    assert_eq!(index.get(0).unwrap(), user);

    let position = client.get_position(&user);
    assert_eq!(position.collateral.len(), 1);
    assert_eq!(position.collateral.get(0).unwrap().asset, token_id);
    assert_eq!(position.collateral.get(0).unwrap().amount, 500);
}

#[test]
fn test_deposit_second_asset_appends_to_position() {
    let (env, client, _admin, user, _oracle, token_id1, _token_client1, token_admin1) = setup_env();

    let token_admin2 = Address::generate(&env);
    let token_contract2 = env.register_stellar_asset_contract_v2(token_admin2);
    let token_id2 = token_contract2.address();
    let token_admin_client2 = token::StellarAssetClient::new(&env, &token_id2);

    client.add_supported_asset(&token_id2);

    token_admin1.mint(&user, &1000);
    token_admin_client2.mint(&user, &1000);

    client.deposit(&user, &token_id1, &500);
    client.deposit(&user, &token_id2, &300);

    assert_eq!(client.get_position_balance(&user, &token_id1), 500);
    assert_eq!(client.get_position_balance(&user, &token_id2), 300);

    let position = client.get_position(&user);
    assert_eq!(position.collateral.len(), 2);
}

#[test]
fn test_deposit_unsupported_asset_fails() {
    let (env, client, _admin, user, _oracle, _token_id, _token_client, _token_admin) = setup_env();

    let other_token_admin = Address::generate(&env);
    let other_token = env.register_stellar_asset_contract_v2(other_token_admin);
    let other_token_id = other_token.address();
    let other_token_admin_client = token::StellarAssetClient::new(&env, &other_token_id);

    other_token_admin_client.mint(&user, &1000);
    let res = client.try_deposit(&user, &other_token_id, &500);
    assert!(res.is_err());
}

#[test]
fn test_deposit_when_paused_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.pause_operation(&PauseFlag::Deposit, &Symbol::new(&env, "test"));

    let res = client.try_deposit(&user, &token_id, &500);
    assert!(res.is_err());
}

#[test]
fn test_deposit_without_auth_fails() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);

    // Disable mock auths after setup so this call must satisfy user.require_auth().
    env.set_auths(&[]);

    let res = client.try_deposit(&user, &token_id, &500);
    assert!(res.is_err());
}

#[test]
fn test_deposit_emits_event() {
    let (env, client, _admin, user, _oracle, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);

    use soroban_sdk::TryFromVal;
    let event_symbol =
        soroban_sdk::Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(event_symbol, soroban_sdk::Symbol::new(&env, "deposited"));

    // With #[contractevent], data is a map/struct Val; verify fields via the topics symbol
    // and confirm the event was emitted from the correct contract (already checked above).
    // The symbol topic check above is sufficient to confirm the correct event type was emitted.
}

#[test]
fn test_deposit_token_transfer() {
    let (_env, client, _admin, user, _oracle, token_id, token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);

    assert_eq!(token_client.balance(&user), 1000);
    assert_eq!(token_client.balance(&client.address), 0);

    client.deposit(&user, &token_id, &500);

    assert_eq!(token_client.balance(&user), 500);
    assert_eq!(token_client.balance(&client.address), 500);
}
