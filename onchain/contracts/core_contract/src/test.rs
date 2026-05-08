#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{StellarAssetClient, TokenClient},
    Address, BytesN, Env, String, Symbol,
};

use crate::{types::ChainType, CoreContract, CoreContractClient};

fn setup_core<'a>(env: &'a Env, owner: &Address) -> (CoreContractClient<'a>, BytesN<32>) {
    let username_hash: BytesN<32> = BytesN::from_array(env, &[1u8; 32]);
    let contract_id = env.register(CoreContract, (owner.clone(), username_hash.clone()));
    let client = CoreContractClient::new(env, &contract_id);
    (client, username_hash)
}

fn setup_token(env: &Env, admin: &Address) -> Address {
    let asset_id = env.register_stellar_asset_contract_v2(admin.clone());
    asset_id.address()
}
#[test]
fn test_constructor_stores_owner_and_hash() {
    let env = Env::default();
    let owner = Address::generate(&env);
    let (client, hash) = setup_core(&env, &owner);

    assert_eq!(client.get_owner(), Some(owner));
    assert_eq!(client.get_username_hash(), Some(hash));
}

#[test]
fn test_set_and_resolve_primary_address() {
    let env = Env::default();
    env.mock_all_auths();
    let owner = Address::generate(&env);
    let (client, _) = setup_core(&env, &owner);

    let recv = Address::generate(&env);
    client.set_primary_address(&recv);

    assert_eq!(client.get_primary_address(), Some(recv.clone()));
    assert_eq!(client.resolve(), recv);
}

#[test]
#[should_panic]
fn test_resolve_without_primary_panics() {
    let env = Env::default();
    let owner = Address::generate(&env);
    let (client, _) = setup_core(&env, &owner);
    client.resolve();
}

#[test]
fn test_wallet_book_add_get_remove() {
    let env = Env::default();
    env.mock_all_auths();
    let owner = Address::generate(&env);
    let (client, _) = setup_core(&env, &owner);

    let label = Symbol::new(&env, "binance");
    let addr_str = String::from_str(&env, "0xABCDEF1234567890");

    client.add_wallet(&label, &addr_str, &ChainType::Ethereum);

    let entry = client.get_wallet(&label).expect("wallet should exist");
    assert_eq!(entry.label, label);
    assert_eq!(entry.address, addr_str);

    client.remove_wallet(&label);
    assert!(client.get_wallet(&label).is_none());
}

#[test]
fn test_get_all_wallets() {
    let env = Env::default();
    env.mock_all_auths();
    let owner = Address::generate(&env);
    let (client, _) = setup_core(&env, &owner);

    client.add_wallet(
        &Symbol::new(&env, "bybit"),
        &String::from_str(&env, "TRX_addr_123"),
        &ChainType::Tron,
    );
    client.add_wallet(
        &Symbol::new(&env, "binance"),
        &String::from_str(&env, "0xEVMaddr"),
        &ChainType::Ethereum,
    );

    let all = client.get_all_wallets();
    assert_eq!(all.len(), 2);
}

#[test]
fn test_transfer_ownership() {
    let env = Env::default();
    env.mock_all_auths();
    let owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let (client, _) = setup_core(&env, &owner);

    client.transfer_ownership(&new_owner);
    assert_eq!(client.get_owner(), Some(new_owner));
}

#[test]
fn test_send_to_address() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (client, _) = setup_core(&env, &owner);

    let token_admin = Address::generate(&env);
    let token_addr = setup_token(&env, &token_admin);
    let token_client = TokenClient::new(&env, &token_addr);
    let sac = StellarAssetClient::new(&env, &token_addr);

    sac.mint(&owner, &1000);
    assert_eq!(token_client.balance(&owner), 1000);

    client.send_to_address(&token_addr, &500, &recipient);

    assert_eq!(token_client.balance(&owner), 500);
    assert_eq!(token_client.balance(&recipient), 500);
}

#[test]
fn test_escrow_create_and_release() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (client, _) = setup_core(&env, &owner);

    let token_admin = Address::generate(&env);
    let token_addr = setup_token(&env, &token_admin);
    let token_client = TokenClient::new(&env, &token_addr);
    let sac = StellarAssetClient::new(&env, &token_addr);
    sac.mint(&owner, &1000);

    env.ledger().with_mut(|l| l.timestamp = 1000);

    let release_at: u64 = 2000;
    let note = String::from_str(&env, "test escrow");
    let id = client.create_escrow(&token_addr, &300, &recipient, &release_at, &note);

    assert_eq!(token_client.balance(&owner), 700);

    env.ledger().with_mut(|l| l.timestamp = 3000);
    client.release_escrow(&id);

    assert_eq!(token_client.balance(&recipient), 300);
}

#[test]
fn test_escrow_refund() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let recipient = Address::generate(&env);
    let (client, _) = setup_core(&env, &owner);

    let token_admin = Address::generate(&env);
    let token_addr = setup_token(&env, &token_admin);
    let sac = StellarAssetClient::new(&env, &token_addr);
    let token_client = TokenClient::new(&env, &token_addr);
    sac.mint(&owner, &1000);

    env.ledger().with_mut(|l| l.timestamp = 1000);

    let id = client.create_escrow(
        &token_addr,
        &400,
        &recipient,
        &9999,
        &String::from_str(&env, "refund test"),
    );

    client.refund_escrow(&id);
    assert_eq!(token_client.balance(&owner), 1000);
}
