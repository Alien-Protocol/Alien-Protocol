#![cfg(test)]

use super::super::*;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{contract, contractimpl, token, Address, Env};

#[contract]
pub struct MockVaultContract;

#[contractimpl]
impl MockVaultContract {
    pub fn get_position(env: Env, user: Address) -> shared::Position {
        let mut collateral = soroban_sdk::Vec::new(&env);
        collateral.push_back(shared::CollateralAsset {
            asset: Address::generate(&env),
            amount: 10_000_000,
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
        100_000_000
    }

    pub fn get_health_factor(_env: Env, _user: Address) -> i128 {
        15_000
    }
}

fn setup_env() -> (
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
    env.ledger().set_timestamp(1_000);

    let contract_id = env.register(PoolContract, ());
    let client = PoolContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let vault_id = env.register(MockVaultContract, ());
    let oracle = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_id = token_contract.address();
    let token_client = token::Client::new(&env, &token_id);
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);

    let interest_rate_bps = 1_000; // 10%
    client.initialize(&admin, &vault_id, &oracle, &token_id, &interest_rate_bps);

    // Seed pool with deep liquidity for borrows
    let liquidity_provider = Address::generate(&env);
    token_admin_client.mint(&liquidity_provider, &500_000_000);
    client.supply(&liquidity_provider, &500_000_000);

    (
        env,
        client,
        admin,
        user,
        token_id,
        token_client,
        token_admin_client,
    )
}

#[test]
fn prop_borrow_never_exceeds_calculate_limit() {
    let (_env, client, _admin, user, token_id, _token_client, _token_admin) = setup_env();

    let limit = client.calculate_limit(&user);
    assert_eq!(limit, 80_000_000); // 100M * 80% LTV = 80M

    // Over-limit borrows must strictly fail
    let over_amounts = [limit + 1, limit + 100, limit * 2, i128::MAX];
    for amount in over_amounts {
        let res = client.try_borrow(&user, &token_id, &amount);
        assert!(
            res.is_err(),
            "Borrow exceeding limit must fail for amount {amount}"
        );
        let debt = client.get_user_debt(&user);
        assert!(
            debt <= limit,
            "Debt must never exceed calculate_limit after failed borrow"
        );
    }

    // Within-limit borrows must succeed and total debt must not exceed limit
    let valid_borrow = 50_000_000;
    let res = client.try_borrow(&user, &token_id, &valid_borrow);
    assert!(res.is_ok(), "Borrow within limit must succeed");

    let debt = client.get_user_debt(&user);
    assert!(
        debt <= limit,
        "Total debt must stay within limit: debt={debt}, limit={limit}"
    );

    // A second borrow that would push total debt over limit must fail
    let push_over = 35_000_000; // 50M + 35M = 85M > 80M
    let res_second = client.try_borrow(&user, &token_id, &push_over);
    assert!(
        res_second.is_err(),
        "Second borrow exceeding limit must fail"
    );
    let debt_after = client.get_user_debt(&user);
    assert!(
        debt_after <= limit,
        "Debt must remain within limit after rejected second borrow"
    );
}

#[test]
fn prop_full_repay_clears_debt() {
    let (_env, client, _admin, user, token_id, _token_client, token_admin) = setup_env();

    let borrow_amounts = [1_000i128, 50_000, 1_000_000, 10_000_000, 40_000_000];

    for amount in borrow_amounts {
        // Borrow amount
        let res_borrow = client.try_borrow(&user, &token_id, &amount);
        assert!(res_borrow.is_ok(), "Borrow must succeed for {amount}");

        let debt_before = client.get_user_debt(&user);
        assert!(debt_before >= amount);

        // Mint extra to user to guarantee full repayment capacity
        token_admin.mint(&user, &debt_before);

        // Repay full debt
        let res_repay = client.try_repay(&user, &token_id, &debt_before);
        assert!(res_repay.is_ok(), "Repay must succeed for {debt_before}");

        let debt_after = client.get_user_debt(&user);
        assert_eq!(
            debt_after, 0,
            "Full repay must completely clear debt for amount {amount}"
        );
    }
}
