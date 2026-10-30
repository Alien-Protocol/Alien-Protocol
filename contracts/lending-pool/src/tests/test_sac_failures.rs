#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, IssuerFlags};
use soroban_sdk::{contract, contractimpl, token, Address, Env};

#[contract]
pub struct MockVaultContract;

#[contractimpl]
impl MockVaultContract {
    pub fn get_health_factor(_env: Env, _user: Address) -> i128 {
        15_000
    }

    pub fn get_collateral_value(_env: Env, _user: Address) -> i128 {
        100_000
    }

    pub fn get_position(env: Env, user: Address) -> shared::Position {
        let mut collateral = soroban_sdk::Vec::new(&env);
        collateral.push_back(shared::CollateralAsset {
            asset: Address::generate(&env),
            amount: 100_000,
        });
        shared::Position { user, collateral }
    }

    pub fn get_asset_config(_env: Env, _asset: Address) -> shared::AssetConfig {
        shared::AssetConfig {
            token_decimals: 7,
            oracle_price_decimals: 8,
            max_ltv_bps: 8000,
            liquidation_threshold_bps: 8500,
        }
    }
}

fn setup_sac_pool_env() -> (
    Env,
    PoolContractClient<'static>,
    Address,
    Address,
    Address,
    token::Client<'static>,
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PoolContract, ());
    let client = PoolContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let oracle = Address::generate(&env);
    let vault_id = env.register(MockVaultContract, ());

    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin);
    sac.issuer().set_flag(IssuerFlags::RevocableFlag);
    sac.issuer().set_flag(IssuerFlags::ClawbackEnabledFlag);

    let borrow_asset = sac.address();
    let token_client = token::Client::new(&env, &borrow_asset);
    let token_admin_client = token::StellarAssetClient::new(&env, &borrow_asset);

    let interest_rate_bps = 1000;
    client.initialize(
        &admin,
        &vault_id,
        &oracle,
        &borrow_asset,
        &interest_rate_bps,
    );

    (
        env,
        client,
        admin,
        user,
        borrow_asset,
        token_client,
        token_admin_client,
    )
}

#[test]
fn test_supply_fails_when_borrow_asset_frozen() {
    let (_env, client, _admin, user, _borrow_asset, _token_client, token_admin) =
        setup_sac_pool_env();

    token_admin.mint(&user, &1000);
    // Freeze user so pool supply cannot pull borrow asset
    token_admin.set_authorized(&user, &false);

    let res = client.try_supply(&user, &500);
    assert_eq!(res, Err(Ok(PoolError::TransferFailed)));

    // Invariant: Failed pull does not increase user supply or total supply
    assert_eq!(client.get_user_supply(&user), 0);
    assert_eq!(client.get_total_supply(), 0);
}

#[test]
fn test_borrow_fails_when_user_cannot_receive_borrow_asset() {
    let (env, client, _admin, user, borrow_asset, _token_client, token_admin) =
        setup_sac_pool_env();

    // Supply initial pool liquidity from supplier
    let supplier = Address::generate(&env);
    token_admin.mint(&supplier, &10_000);
    client.supply(&supplier, &10_000);

    // Freeze user on borrow asset SAC so transfer out fails
    token_admin.set_authorized(&user, &false);

    let res = client.try_borrow(&user, &borrow_asset, &500);
    assert_eq!(res, Err(Ok(PoolError::TransferFailed)));

    // Invariant: Pool debt and liquidity must not desync or change
    assert_eq!(client.get_user_debt(&user), 0);
    assert_eq!(client.get_available_liquidity(), 10_000);
}

#[test]
fn test_repay_fails_when_pool_cannot_pull_borrow_asset() {
    let (env, client, _admin, user, borrow_asset, _token_client, token_admin) =
        setup_sac_pool_env();

    // Supply liquidity to pool
    let supplier = Address::generate(&env);
    token_admin.mint(&supplier, &10_000);
    client.supply(&supplier, &10_000);

    // User borrows 500 when active
    token_admin.mint(&user, &1000);
    client.borrow(&user, &borrow_asset, &500);
    assert_eq!(client.get_user_debt(&user), 500);

    // Freeze user on borrow asset SAC so pool cannot pull repayment
    token_admin.set_authorized(&user, &false);

    let res = client.try_repay(&user, &borrow_asset, &500);
    assert_eq!(res, Err(Ok(PoolError::TransferFailed)));

    // Invariant: Debt remains unchanged
    assert_eq!(client.get_user_debt(&user), 500);
}
