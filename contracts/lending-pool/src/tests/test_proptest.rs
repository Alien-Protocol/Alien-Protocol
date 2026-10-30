#![cfg(test)]

use super::super::*;
use proptest::prelude::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{contract, contractimpl, token, Address, Env};

#[contract]
pub struct MockPropVaultContract;

#[contractimpl]
impl MockPropVaultContract {
    pub fn get_position(env: Env, user: Address) -> shared::Position {
        let mut collateral = soroban_sdk::Vec::new(&env);
        collateral.push_back(shared::CollateralAsset {
            asset: Address::generate(&env),
            amount: 1_000_000_000_000,
        });
        shared::Position { user, collateral }
    }

    pub fn get_asset_config(_env: Env, _asset: Address) -> shared::AssetConfig {
        shared::AssetConfig {
            token_decimals: 7,
            oracle_price_decimals: 7,
            max_ltv_bps: 8_000,
            liquidation_threshold_bps: 8_500,
        }
    }

    pub fn get_collateral_value(_env: Env, _user: Address) -> i128 {
        1_000_000_000_000
    }

    pub fn get_health_factor(_env: Env, _user: Address) -> i128 {
        15_000
    }
}

fn setup_pool_env() -> (
    Env,
    PoolContractClient<'static>,
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
    let vault = env.register(MockPropVaultContract, ());

    let token_admin = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(token_admin);
    let borrow_asset = sac.address();
    let token_client = token::Client::new(&env, &borrow_asset);
    let token_admin_client = token::StellarAssetClient::new(&env, &borrow_asset);

    client.initialize(&admin, &vault, &oracle, &borrow_asset, &500);

    // Supplier supplies deep liquidity
    let supplier = Address::generate(&env);
    token_admin_client.mint(&supplier, &10_000_000_000_000);
    client.supply(&supplier, &10_000_000_000_000);

    (
        env,
        client,
        user,
        borrow_asset,
        token_client,
        token_admin_client,
    )
}

proptest! {
    #[test]
    fn prop_full_repay_clears_debt(
        borrow_amount in shared::MIN_REMAINING_DEBT..=1_000_000_000i128,
    ) {
        let (_env, client, user, asset, _token_client, token_admin) = setup_pool_env();

        client.borrow(&user, &asset, &borrow_amount);
        let debt_before = client.get_user_debt(&user);
        prop_assert_eq!(debt_before, borrow_amount);

        // Mint necessary repayment to user and execute full repay
        token_admin.mint(&user, &debt_before);
        client.repay(&user, &asset, &debt_before);

        let debt_after = client.get_user_debt(&user);
        prop_assert_eq!(debt_after, 0);
    }

    #[test]
    fn prop_borrow_never_exceeds_calculate_limit(
        excess in 1i128..=10_000_000i128,
    ) {
        let (env, client, user, asset, _token_client, _token_admin) = setup_pool_env();

        let limit = env.as_contract(&client.address, || {
            borrow::calculate_limit(env.clone(), user.clone()).unwrap()
        });
        let over_limit = limit.saturating_add(excess);

        let res = client.try_borrow(&user, &asset, &over_limit);
        prop_assert!(res.is_err());
    }
}
