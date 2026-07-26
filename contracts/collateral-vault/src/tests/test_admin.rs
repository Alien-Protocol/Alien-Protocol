use super::super::*;
use soroban_sdk::testutils::{Address as _, Events};
use soroban_sdk::{token, Address, Env};

fn setup_env() -> (
    Env,
    VaultContractClient<'static>,
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
        token_contract_id,
        token_client,
        token_admin_client,
    )
}

#[test]
fn test_propose_admin_success() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.propose_admin(&new_admin);

    assert_eq!(client.get_pending_admin(), Some(new_admin));
}

#[test]
fn test_propose_admin_emits_event() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.propose_admin(&new_admin);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);
    use soroban_sdk::TryFromVal;
    let event_symbol =
        soroban_sdk::Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(
        event_symbol,
        soroban_sdk::Symbol::new(&env, "admin_proposed")
    );
}

#[test]
fn test_accept_admin_success() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.propose_admin(&new_admin);
    client.accept_admin();

    assert_eq!(client.get_admin(), Some(new_admin));
    assert_eq!(client.get_pending_admin(), None);
}

#[test]
fn test_accept_admin_emits_event() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.propose_admin(&new_admin);
    client.accept_admin();

    let all_events = env.events().all();
    let len = all_events.len();
    let accept_event = all_events.get(len - 1).unwrap();
    assert_eq!(accept_event.0, client.address);
    use soroban_sdk::TryFromVal;
    let event_symbol =
        soroban_sdk::Symbol::try_from_val(&env, &accept_event.1.get(0).unwrap()).unwrap();
    assert_eq!(
        event_symbol,
        soroban_sdk::Symbol::new(&env, "admin_accepted")
    );
}

#[test]
fn test_accept_admin_no_pending_fails() {
    let (_env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let res = client.try_accept_admin();
    assert!(res.is_err());
}

#[test]
fn test_cancel_admin_transfer_success() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.propose_admin(&new_admin);
    client.cancel_admin_transfer();

    assert_eq!(client.get_pending_admin(), None);

    let res = client.try_accept_admin();
    assert!(res.is_err());
}

#[test]
fn test_cancel_admin_transfer_emits_event() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.propose_admin(&new_admin);
    client.cancel_admin_transfer();

    let all_events = env.events().all();
    let len = all_events.len();
    let cancel_event = all_events.get(len - 1).unwrap();
    assert_eq!(cancel_event.0, client.address);
    use soroban_sdk::TryFromVal;
    let event_symbol =
        soroban_sdk::Symbol::try_from_val(&env, &cancel_event.1.get(0).unwrap()).unwrap();
    assert_eq!(
        event_symbol,
        soroban_sdk::Symbol::new(&env, "admin_transfer_cancelled")
    );
}

#[test]
fn test_cancel_admin_transfer_no_pending_fails() {
    let (_env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let res = client.try_cancel_admin_transfer();
    assert!(res.is_err());
}

#[test]
fn test_propose_admin_same_address() {
    let (_env, client, admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let result = client.try_propose_admin(&admin);
    assert_eq!(result, Err(Ok(VaultError::AlreadyAdmin)));
}

#[test]
fn test_full_admin_lifecycle() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let new_admin = Address::generate(&env);
    let wrong_user = Address::generate(&env);

    client.propose_admin(&new_admin);
    assert_eq!(client.get_pending_admin(), Some(new_admin));

    client.accept_admin();
    assert_eq!(client.get_admin(), Some(new_admin));
    assert_eq!(client.get_pending_admin(), None);

    let second_admin = Address::generate(&env);
    client.propose_admin(&second_admin);
    assert_eq!(client.get_pending_admin(), Some(second_admin));

    client.cancel_admin_transfer();
    assert_eq!(client.get_pending_admin(), None);
    assert_eq!(client.get_admin(), Some(new_admin));
}

#[test]
fn test_old_admin_cannot_act_after_transfer() {
    let (env, client, admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.propose_admin(&new_admin);
    client.accept_admin();

    assert_eq!(client.get_admin(), Some(new_admin));
    assert_ne!(client.get_admin(), Some(admin));
}

#[test]
fn test_admin_can_act_before_transfer_accepted() {
    let (env, client, _admin, user, token_id, _token_client, token_admin) = setup_env();

    let new_admin = Address::generate(&env);
    client.propose_admin(&new_admin);

    token_admin.mint(&user, &1000);
    client.pause();

    let res = client.try_deposit(&user, &token_id, &500);
    assert!(res.is_err());
}

#[test]
fn test_grant_role_success() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let role = Role::EmergencyPauser;
    let operator = Address::generate(&env);
    client.grant_role(&role, &operator);
}

#[test]
fn test_grant_role_emits_event() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let role = Role::EmergencyPauser;
    let operator = Address::generate(&env);
    client.grant_role(&role, &operator);

    let all_events = env.events().all();
    let len = all_events.len();
    let grant_event = all_events.get(len - 1).unwrap();
    assert_eq!(grant_event.0, client.address);
    use soroban_sdk::TryFromVal;
    let event_symbol =
        soroban_sdk::Symbol::try_from_val(&env, &grant_event.1.get(0).unwrap()).unwrap();
    assert_eq!(event_symbol, soroban_sdk::Symbol::new(&env, "role_granted"));
}

#[test]
fn test_revoke_role_success() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let role = Role::EmergencyPauser;
    let operator = Address::generate(&env);
    client.grant_role(&role, &operator);
    client.revoke_role(&role, &operator);
}

#[test]
fn test_revoke_role_emits_event() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let role = Role::EmergencyPauser;
    let operator = Address::generate(&env);
    client.grant_role(&role, &operator);
    client.revoke_role(&role, &operator);

    let all_events = env.events().all();
    let len = all_events.len();
    let revoke_event = all_events.get(len - 1).unwrap();
    assert_eq!(revoke_event.0, client.address);
    use soroban_sdk::TryFromVal;
    let event_symbol =
        soroban_sdk::Symbol::try_from_val(&env, &revoke_event.1.get(0).unwrap()).unwrap();
    assert_eq!(event_symbol, soroban_sdk::Symbol::new(&env, "role_revoked"));
}

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

    let res = client.try_deposit(&user, &token_id, &500);
    assert!(res.is_err());
}

#[test]
fn test_pause_blocks_withdraw() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);

    client.pause();

    let res = client.try_withdraw(&user, &token_id, &100);
    assert!(res.is_err());
}

#[test]
fn test_double_pause_fails() {
    let (_env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    client.pause();
    let res = client.try_pause();
    assert!(res.is_err());
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
fn test_unpause_when_not_paused_fails() {
    let (_env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let res = client.try_unpause();
    assert!(res.is_err());
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

#[test]
fn test_remove_supported_asset_success() {
    let (_env, client, _admin, _user, token_id, _token_client, _token_admin) = setup_env();

    assert!(client.is_supported_asset(&token_id));
    client.remove_supported_asset(&token_id);
    assert!(!client.is_supported_asset(&token_id));
}

#[test]
fn test_remove_supported_asset_non_existent_fails() {
    let (env, client, _admin, _user, _token_id, _token_client, _token_admin) = setup_env();

    let fake_asset = Address::generate(&env);
    let res = client.try_remove_supported_asset(&fake_asset);
    assert!(res.is_err());
}

#[test]
fn test_remove_supported_asset_blocks_deposit() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin) = setup_env();

    token_admin.mint(&user, &1000);
    client.remove_supported_asset(&token_id);

    let res = client.try_deposit(&user, &token_id, &500);
    assert!(res.is_err());
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

#[test]
fn test_remove_supported_asset_emits_event() {
    let (env, client, _admin, _user, token_id, _token_client, _token_admin) = setup_env();

    client.remove_supported_asset(&token_id);

    let last_event = env.events().all().last().unwrap();
    assert_eq!(last_event.0, client.address);
    use soroban_sdk::TryFromVal;
    let event_symbol =
        soroban_sdk::Symbol::try_from_val(&env, &last_event.1.get(0).unwrap()).unwrap();
    assert_eq!(
        event_symbol,
        soroban_sdk::Symbol::new(&env, "asset_removed")
    );
}
