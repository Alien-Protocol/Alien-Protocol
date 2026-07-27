#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::{token, Address, Env};

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
    Address, // admin
    Address, // user
    Address, // token_id
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
        token_contract_id,
        token_client,
        token_admin_client,
    )
}

// ── set_admin ───────────────────────────────────────────────────────────────

#[test]
fn test_set_admin_success() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    assert_eq!(client.get_admin(), Some(new_admin));
}

#[test]
fn test_set_admin_requires_current_admin_auth() {
    let (env, client, admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, admin);
}

#[test]
fn test_set_admin_same_address_fails_with_already_admin() {
    let (env, client, admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    // Calling set_admin with the same address must return AlreadyAdmin.
    let err = client.try_set_admin(&admin).unwrap_err().unwrap();
    assert_eq!(err, VaultError::AlreadyAdmin);
}

#[test]
fn test_set_admin_emits_event() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);
    use soroban_sdk::TryFromVal;
    let event_symbol =
        soroban_sdk::Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(
        event_symbol,
        soroban_sdk::Symbol::new(&env, "admin_changed")
    );
}

#[test]
fn test_old_admin_cannot_act_after_transfer() {
    let (env, client, admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.set_admin(&new_admin);

    // Under mock_all_auths the vault accepts any signer, but it calls
    // require_auth on whichever address is stored as admin — now new_admin.
    client.pause();

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    let (auth_addr, _) = auths.first().unwrap();
    assert_eq!(*auth_addr, new_admin);
    assert_ne!(*auth_addr, admin);
}

// ── pause / unpause ─────────────────────────────────────────────────────────

#[test]
fn test_pause_success() {
    let (_env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();
    client.pause();
}

#[test]
fn test_pause_blocks_deposit() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.pause();

    let err = client.try_deposit(&user, &token_id, &500).unwrap_err().unwrap();
    assert_eq!(err, VaultError::VaultPaused);
}

#[test]
fn test_pause_blocks_withdraw() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);
    client.pause();

    let err = client
        .try_withdraw(&user, &token_id, &100)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, VaultError::VaultPaused);
}

#[test]
fn test_double_pause_fails_with_already_paused() {
    let (_env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    client.pause();
    let err = client.try_pause().unwrap_err().unwrap();
    assert_eq!(err, VaultError::AlreadyPaused);
}

#[test]
fn test_unpause_success() {
    let (_env, client, _admin, user, token_id, token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.pause();
    client.unpause();

    client.deposit(&user, &token_id, &500);
    assert_eq!(token_client.balance(&user), 500);
}

#[test]
fn test_unpause_when_not_paused_fails_with_not_paused() {
    let (_env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let err = client.try_unpause().unwrap_err().unwrap();
    assert_eq!(err, VaultError::NotPaused);
}

#[test]
fn test_unpause_emits_event() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    client.pause();
    client.unpause();

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);
    use soroban_sdk::TryFromVal;
    let event_symbol =
        soroban_sdk::Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(event_symbol, soroban_sdk::Symbol::new(&env, "unpaused"));
}

// ── supported-asset management ──────────────────────────────────────────────

#[test]
fn test_remove_supported_asset_success() {
    let (_env, client, _admin, _user, token_id, _token_client, _token_admin) = setup_env();

    assert!(client.is_supported_asset(&token_id));
    client.remove_supported_asset(&token_id);
    assert!(!client.is_supported_asset(&token_id));
}

#[test]
fn test_remove_supported_asset_non_existent_fails_with_asset_not_found() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let fake_asset = Address::generate(&env);
    let err = client
        .try_remove_supported_asset(&fake_asset)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, VaultError::AssetNotFound);
}

#[test]
fn test_remove_supported_asset_blocks_deposit() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.remove_supported_asset(&token_id);

    let err = client
        .try_deposit(&user, &token_id, &500)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, VaultError::UnsupportedAsset);
}

#[test]
fn test_remove_supported_asset_keeps_existing_positions() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);
    client.remove_supported_asset(&token_id);

    let position = client.get_position(&user);
    assert_eq!(position.collateral.len(), 1);
    assert_eq!(position.collateral.get(0).unwrap().amount, 500);
}
