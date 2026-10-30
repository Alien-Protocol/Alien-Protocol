#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, IssuerFlags};
use soroban_sdk::{contract, contractimpl, token, Address, Env};

#[contract]
pub struct MockLendingPool;

#[contractimpl]
impl MockLendingPool {
    pub fn get_user_debt(_env: Env, _user: Address) -> i128 {
        0
    }

    pub fn is_liquidatable(_env: Env, _user: Address) -> bool {
        true
    }
}

fn setup_sac_env() -> (
    Env,
    VaultContractClient<'static>,
    Address,
    Address,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(VaultContract, ());
    let client = VaultContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let oracle = Address::generate(&env);
    let lending_pool = env.register(MockLendingPool, ());
    let liquidation_engine = Address::generate(&env);

    client.initialize(&admin, &lending_pool, &oracle, &liquidation_engine);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    token_contract.issuer().set_flag(IssuerFlags::RevocableFlag);
    token_contract
        .issuer()
        .set_flag(IssuerFlags::ClawbackEnabledFlag);

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
        liquidation_engine,
    )
}

#[test]
fn test_deposit_fails_when_trustline_missing() {
    let (env, client, _admin, _user, _oracle, token_id, _token_client, _token_admin, _engine) =
        setup_sac_env();

    // G-address classic account with no trustline established for the SAC
    let g_user = Address::from_str(
        &env,
        "GBRPYHIL2CI3FNQ4BXLFMNDLFJUNPU2HY3ZMFSHONUCEOASW7QC7OX2H",
    );
    let res = client.try_deposit(&g_user, &token_id, &500);
    assert_eq!(
        res,
        Err(Ok(soroban_sdk::Error::from_contract_error(
            VaultError::TransferFailed as u32
        )))
    );
}

#[test]
fn test_deposit_fails_when_trustline_deauthorized() {
    let (_env, client, _admin, user, _oracle, token_id, _token_client, token_admin, _engine) =
        setup_sac_env();

    token_admin.mint(&user, &1000);
    // Deauthorize / freeze user on SAC
    token_admin.set_authorized(&user, &false);

    let res = client.try_deposit(&user, &token_id, &500);
    assert_eq!(
        res,
        Err(Ok(soroban_sdk::Error::from_contract_error(
            VaultError::TransferFailed as u32
        )))
    );
}

#[test]
fn test_failed_deposit_does_not_credit_position() {
    let (_env, client, _admin, user, _oracle, token_id, _token_client, token_admin, _engine) =
        setup_sac_env();

    token_admin.mint(&user, &1000);
    // Freeze user so deposit transfer fails
    token_admin.set_authorized(&user, &false);

    let res = client.try_deposit(&user, &token_id, &500);
    assert_eq!(
        res,
        Err(Ok(soroban_sdk::Error::from_contract_error(
            VaultError::TransferFailed as u32
        )))
    );

    // Fail-closed invariant: position balance must not increase / credit must not persist
    assert_eq!(client.get_position_balance(&user, &token_id), 0);
}

#[test]
fn test_withdraw_fails_when_destination_cannot_receive() {
    let (_env, client, _admin, user, _oracle, token_id, _token_client, token_admin, _engine) =
        setup_sac_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);
    assert_eq!(client.get_position_balance(&user, &token_id), 500);

    // Freeze user destination on SAC so push fails
    token_admin.set_authorized(&user, &false);

    let res = client.try_withdraw(&user, &token_id, &200);
    assert_eq!(
        res,
        Err(Ok(soroban_sdk::Error::from_contract_error(
            VaultError::TransferFailed as u32
        )))
    );

    // Invariant: Failed SAC push does not decrease vault accounting without a matching revert
    assert_eq!(client.get_position_balance(&user, &token_id), 500);
}

#[test]
fn test_seize_fails_when_engine_cannot_receive_asset() {
    let (_env, client, _admin, user, _oracle, token_id, _token_client, token_admin, engine) =
        setup_sac_env();

    token_admin.mint(&user, &1000);
    client.deposit(&user, &token_id, &500);
    assert_eq!(client.get_position_balance(&user, &token_id), 500);

    // Freeze liquidation engine so it cannot receive the seized asset
    token_admin.set_authorized(&engine, &false);

    let res = client.try_seize_collateral(&engine, &user, &token_id, &200);
    assert_eq!(res, Err(Ok(VaultError::TransferFailed)));

    // Invariant: Seize accounting not decremented when engine transfer fails
    assert_eq!(client.get_position_balance(&user, &token_id), 500);
}
